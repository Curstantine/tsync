use std::{env, fs, io, path::Path};

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    errors::{Error, Result},
    format::{Codec, get_track_data},
    utils::{
        adb_file_exists,
        ffmpeg::{strip_covers, transcode_file},
        fs::{FSBackend, get_file_ext, get_file_name, read_dir_recursively, read_selectively},
        is_adb_running, parse_sync_list, push_to_adb_device,
    },
};

pub struct SyncOpts {
    pub fs: FSBackend,
    pub codec: Option<Codec>,
    pub bitrate: Option<u32>,
    pub strip_covers: bool,
    pub transcode_codecs: Vec<Codec>,
    pub sync_codecs: Vec<Codec>,
    pub sync_list: Option<String>,
}

pub fn run<P: AsRef<Path>>(source_dir: P, target_dir: P, opts: SyncOpts) -> Result<()> {
    let fs_wrapper = FSWrapper::new(opts.fs)?;
    let source_dir = source_dir.as_ref();
    let target_dir = target_dir.as_ref();
    let temp_dir = env::temp_dir().join("tsync");

    let sync_list_files = opts
        .sync_list
        .map(|x| parse_sync_list(source_dir, x.as_ref()))
        .transpose()?;

    if let Err(e) = fs::create_dir(&temp_dir) {
        if e.kind() != io::ErrorKind::AlreadyExists {
            return Err(e.into());
        }

        fs::remove_dir_all(&temp_dir)?;
        fs::create_dir(&temp_dir)?;
    }

    let bitrate = opts
        .codec
        .as_ref()
        .map(|c| c.get_matching_bitrate(opts.bitrate))
        .transpose()?;
    if bitrate.is_some() && opts.transcode_codecs.iter().any(|tc| opts.sync_codecs.contains(tc)) {
        return Err(Error::descriptive("Sync and transcode codecs cannot overlap!"));
    }

    let files = {
        let readable_extensions = if opts.codec.is_some() {
            opts.transcode_codecs
                .iter()
                .chain(opts.sync_codecs.iter())
                .map(|x| x.get_extension_str())
                .collect::<Vec<&'static str>>()
        } else {
            opts.sync_codecs
                .iter()
                .map(|x| x.get_extension_str())
                .collect::<Vec<&'static str>>()
        };

        if let Some(within) = sync_list_files {
            read_selectively(&within, &Some(readable_extensions))?
        } else {
            read_dir_recursively(source_dir, &Some(readable_extensions))?
        }
    };

    println!("Found {} files", files.len().to_string().green());

    let indicator = ProgressBar::new(files.len() as u64).with_style(
        ProgressStyle::with_template("{msg}\n[{elapsed_precise}] [{wide_bar:.cyan/blue}] [{pos}/{len}]")
            .unwrap()
            .progress_chars("#>-"),
    );

    let path_already_exists = |p: &Path, indicator: &ProgressBar| {
        let message = format!("{} already exists", get_file_name(p));
        indicator.set_message(message);
        indicator.inc(1);
    };

    let skipping = |p: &Path, indicator: &ProgressBar, message: Option<&'static str>| {
        let message = format!("Skipping {} {}", get_file_name(p), message.unwrap_or(""));
        indicator.set_message(message);
        indicator.inc(1);
    };

    for file in files.into_iter() {
        let mut rel_path = file.strip_prefix(source_dir).unwrap().to_path_buf();
        let source_file_ext = get_file_ext(file.as_ref());

        // But why? Can't we use the check from codec.is_some()? No, not really.
        // We support syncing files that are part of the sync_extensions, so they don't go through the transcoding workflow.
        // So in cases like removing the temp file, it will remove the source file instead.
        let mut is_temp = false;
        let mut final_source_path = file.clone();

        let meta = get_track_data(&file, &source_file_ext)?;
        let is_transcodable = !opts.sync_codecs.contains(&meta.codec) && opts.transcode_codecs.contains(&meta.codec);

        match &opts.codec {
            Some(codec) if is_transcodable => {
                let new_ext = codec.get_extension_str();
                let temp_path = temp_dir.join(&rel_path).with_extension(new_ext);
                let bitrate = bitrate.expect("Bitrate must be set if codec is set");

                if fs_wrapper.exists(&target_dir.join(rel_path.with_extension(new_ext)))? {
                    path_already_exists(&rel_path, &indicator);
                    continue;
                }

                let message = format!("Transcoding {n} [{codec:?}@{bitrate}K]", n = get_file_name(&rel_path));
                indicator.set_message(message);

                fs::create_dir_all(temp_path.parent().unwrap())?;
                transcode_file(&file, &temp_path, *codec, bitrate)?;

                is_temp = true;
                final_source_path = temp_path;
                rel_path.set_extension(new_ext);
            }
            None if is_transcodable => {
                skipping(&rel_path, &indicator, Some("due to no codec"));
                continue;
            }
            _ if opts.sync_codecs.contains(&meta.codec) => {
                if fs_wrapper.exists(&target_dir.join(&rel_path))? {
                    path_already_exists(&rel_path, &indicator);
                    continue;
                }

                if opts.strip_covers {
                    let temp_path = temp_dir.join(&rel_path);
                    fs::create_dir_all(temp_path.parent().unwrap())?;
                    fs::copy(&file, &temp_path)?;

                    let message = format!("Stripping covers from {}", get_file_name(&rel_path));
                    indicator.set_message(message);

                    strip_covers(&temp_path, &temp_path)?;

                    is_temp = true;
                    final_source_path = temp_path;
                }
            }
            _ => unreachable!(),
        }

        indicator.set_message(format!("Syncing {:?}", get_file_name(&rel_path)));
        fs_wrapper.copy(&final_source_path, &target_dir.join(rel_path))?;

        if is_temp {
            fs::remove_file(final_source_path)?;
        }

        indicator.inc(1);
    }

    indicator.finish_with_message("Done!");

    Ok(())
}

struct FSWrapper {
    fs: FSBackend,
}

impl FSWrapper {
    fn new(fs: FSBackend) -> Result<Self> {
        match fs {
            FSBackend::Adb => {
                if !is_adb_running()? {
                    let message = "adb is not running. Please start adb and try again.".to_string();
                    return Err(Error::descriptive(message));
                }
            }
            FSBackend::Ftp => todo!(),
            FSBackend::None => {}
        }

        Ok(Self { fs })
    }

    fn copy(&self, source: &Path, target: &Path) -> Result<()> {
        match self.fs {
            FSBackend::Adb => push_to_adb_device(source, target),
            FSBackend::Ftp => todo!(),
            FSBackend::None => {
                if let Some(p) = target.parent() {
                    fs::create_dir_all(p)?;
                }

                fs::copy(source, target)?;
                Ok(())
            }
        }
    }

    fn exists(&self, path: &Path) -> Result<bool> {
        match self.fs {
            FSBackend::Adb => adb_file_exists(path),
            FSBackend::Ftp => todo!(),
            FSBackend::None => Ok(path.exists()),
        }
    }
}
