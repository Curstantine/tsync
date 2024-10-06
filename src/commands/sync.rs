use std::{
    fs, io,
    path::{Path, PathBuf},
};

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    errors::{Error, Result},
    format::Codec,
    utils::{
        adb_file_exists, fs::FSBackend, is_adb_running, parse_sync_list, push_to_adb_device, read_dir_recursively,
        transcode_file,
    },
};

const TEMP_DIR: &str = "./tmp";

pub struct SyncOpts {
    pub fs_backend: FSBackend,
    pub codec: Option<Codec>,
    pub bitrate: Option<u32>,
    pub transcode_codecs: Vec<Codec>,
    pub sync_codecs: Vec<Codec>,
    pub sync_list: Option<String>,
}

pub fn run<P: AsRef<Path>>(source_dir: P, target_dir: P, opts: SyncOpts) -> Result<()> {
    let sync_list_files = opts
        .sync_list
        .map(|x| parse_sync_list(source_dir.as_ref(), x.as_ref()))
        .transpose()?;

    match opts.fs_backend {
        FSBackend::Adb => run_backend_adb(
            source_dir.as_ref(),
            target_dir.as_ref(),
            opts.codec,
            opts.bitrate,
            opts.transcode_codecs,
            opts.sync_codecs,
            sync_list_files,
        ),
        _ => unimplemented!(),
    }
}

pub fn run_backend_adb(
    source_dir: &Path,
    target_dir: &Path,
    codec: Option<Codec>,
    bitrate: Option<u32>,
    transcode_codecs: Vec<Codec>,
    sync_codecs: Vec<Codec>,
    sync_file_list: Option<Vec<PathBuf>>,
) -> Result<()> {
    if !is_adb_running()? {
        let message = "adb is not running. Please start adb and try again.".to_string();
        return Err(Error::descriptive(message));
    }

    match fs::create_dir(TEMP_DIR) {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_dir_all(TEMP_DIR)?;
            fs::create_dir(TEMP_DIR)?;
        }
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }

    let bitrate = codec.as_ref().map(|c| c.get_matching_bitrate(bitrate)).transpose()?;
    if bitrate.is_some() && transcode_codecs.iter().all(|tc| sync_codecs.contains(tc)) {
        return Err(Error::descriptive("Sync and transcode extensions cannot overlap!"));
    }

    let temp_dir = Path::new(TEMP_DIR);
    let transcode_extensions = transcode_codecs
        .iter()
        .map(|x| x.get_extension_str().to_string())
        .collect::<Vec<_>>();
    let sync_extensions = transcode_codecs
        .iter()
        .map(|x| x.get_extension_str().to_string())
        .collect::<Vec<_>>();

    let readable_extensions = if codec.is_some() {
        transcode_extensions
            .iter()
            .chain(sync_extensions.iter())
            .map(|ext| ext.to_string())
            .collect::<Vec<_>>()
    } else {
        // We can skip over the transcoding extensions if we don't have a codec.
        sync_extensions.iter().map(|ext| ext.to_string()).collect::<Vec<_>>()
    };

    let files = read_dir_recursively(source_dir, &Some(readable_extensions), &sync_file_list)?;
    println!("Found {} files", files.len().to_string().green());

    let indicator = ProgressBar::new(files.len() as u64);
    indicator.set_style(
        ProgressStyle::with_template("{msg}\n[{elapsed_precise}] [{wide_bar:.cyan/blue}] [{pos}/{len}]")
            .unwrap()
            .progress_chars("#>-"),
    );

    let get_file_name = |p: &Path| p.file_name().unwrap().to_string_lossy().to_string();
    let get_extension = |p: &Path| p.extension().unwrap().to_string_lossy().to_string();
    let path_already_exists = |p: &Path, indicator: &ProgressBar| {
        let message = format!("{n} already exists", n = get_file_name(p));
        indicator.set_message(message);
        indicator.inc(1);
    };
    let skipping = |p: &Path, indicator: &ProgressBar| {
        let message = format!("Skipping {:?}", get_file_name(p));
        indicator.set_message(message);
        indicator.inc(1);
    };

    for file in files.into_iter() {
        let mut rel_path = file.strip_prefix(source_dir).unwrap().to_path_buf();
        let source_file_ext = get_extension(file.as_ref());

        // But why? Can't we use the check from codec.is_some()? No, not really.
        // We support syncing files that are part of the sync_extensions, so they don't go through the transcoding workflow.
        // So in cases like removing the temp file, it will remove the source file instead.
        let mut transcoded = false;
        let mut final_source_path = file.clone();

        match &codec {
            Some(codec) if transcode_extensions.contains(&source_file_ext) => {
                let new_ext = codec.get_extension_str();
                let temp_path = temp_dir.join(&rel_path).with_extension(new_ext);
                let bitrate = bitrate.expect("Bitrate must be set if codec is set");

                // Memory moment. We need to skip over files that already exist on the device.
                let a = target_dir.join(rel_path.with_extension(new_ext));
                if adb_file_exists(&a)? {
                    path_already_exists(&rel_path, &indicator);
                    continue;
                }

                let message = format!("Transcoding {n} [{codec:?}@{bitrate}K]", n = get_file_name(&rel_path));
                indicator.set_message(message);

                fs::create_dir_all(temp_path.parent().unwrap())?;
                transcode_file(&file, &temp_path, *codec, bitrate)?;

                transcoded = true;
                final_source_path = temp_path;
                rel_path.set_extension(new_ext);
            }
            None if transcode_extensions.contains(&source_file_ext) => {
                skipping(&rel_path, &indicator);
                continue;
            }
            _ if sync_extensions.contains(&source_file_ext) => {
                if adb_file_exists(&target_dir.join(&rel_path))? {
                    path_already_exists(&rel_path, &indicator);
                    continue;
                }
            }
            _ => unreachable!(),
        }

        indicator.set_message(format!("Pushing {:?}", get_file_name(&rel_path)));
        let target_path = target_dir.join(rel_path);
        push_to_adb_device(&final_source_path, &target_path)?;

        if transcoded {
            fs::remove_file(final_source_path)?;
        }

        indicator.inc(1);
    }

    indicator.finish_with_message("Done!");

    Ok(())
}
