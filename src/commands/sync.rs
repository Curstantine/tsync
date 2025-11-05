use std::{
    collections::HashSet,
    env, fs, io,
    path::{Path, PathBuf},
};

use clap::{Args, arg};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    errors::{Error, Result},
    format::{Codec, get_track_data},
    utils::{
        ffmpeg::transcode_file,
        fs::{FSBackend, read_dir_recursively, read_selectively},
        parse_sync_list,
        path::PathExtensions,
    },
};

#[derive(Debug, Args)]
pub struct SyncOpts {
    /// The source directory to sync from.
    source: PathBuf,

    /// The directory to sync to.
    target: PathBuf,

    #[arg(long, short, default_value = "adb")]
    /// Specifies the filesystem backend to use for syncing.
    fs: FSBackend,

    #[arg(long, short)]
    /// The codec to transcode into for tracks matching the transcode_codecs.
    ///
    /// Transcoding will only apply if a value is passed, else only the files matched by `sync_extensions` will synced.
    codec: Option<Codec>,

    #[arg(long, short)]
    /// The bitrate to use while transcoding files matched by `transcode_extensions`.
    /// Only applies if `codec` is set.
    ///
    /// Default values:
    /// - opus: 128K
    /// - vorbis: 192K
    /// - mp3: 320K
    /// - aac-lc: 192K
    bitrate: Option<u32>,

    #[arg(long, default_value_t = false)]
    /// When enabled, extras like covers are included with the sync.
    include_extras: bool,

    #[arg(long, value_delimiter = ',', default_value = "flac,alac")]
    /// A comma-separated list of codecs to match to include in the transcode process.
    transcode_codecs: Vec<Codec>,

    #[arg(long, value_delimiter = ',', default_value = "opus,vorbis,mp3,aac-lc")]
    /// A comma-separated list of codecs to match to include only in the sync process.
    sync_codecs: Vec<Codec>,

    #[arg(
        long,
        long_help = "\
A text file containing a list of folders to sync. Folders listed must be exist within the source directory.

E.g. source -> ~/Music/Library:
    ESAI
    ~/Music/Library/K03
    ~/Music/Library/Various Artists/Stream Palette 4
    Various Artists/Stream Palette 5 -RANKED-"
    )]
    sync_list: Option<String>,
}

pub fn run(opts: SyncOpts) -> Result<()> {
    let fs = opts.fs;

    let source_dir = Path::new(&opts.source);
    let target_dir = Path::new(&opts.target);
    let temp_dir = env::temp_dir().join("tsync");

    if !fs.available()? {
        let message = format!("{fs:?} is not available! Make sure everything is right.");
        return Err(Error::descriptive(message));
    }

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
        .map(|c| c.matching_bitrate(opts.bitrate))
        .transpose()?;
    if bitrate.is_some() && opts.transcode_codecs.iter().any(|tc| opts.sync_codecs.contains(tc)) {
        return Err(Error::descriptive("Sync and transcode codecs cannot overlap!"));
    }

    let files = {
        let readable_extensions = if opts.codec.is_some() {
            opts.transcode_codecs
                .iter()
                .chain(opts.sync_codecs.iter())
                .map(|x| x.extenstion_str())
                .collect::<Vec<&'static str>>()
        } else {
            opts.sync_codecs
                .iter()
                .map(|x| x.extenstion_str())
                .collect::<Vec<&'static str>>()
        };

        if let Some(within) = &sync_list_files {
            read_selectively(within, &Some(readable_extensions))?
        } else {
            read_dir_recursively(source_dir, &Some(readable_extensions))?
        }
    };

    println!("Found {} files", files.len().to_string().green());

    let indicator = {
        #[rustfmt::skip]
        let len = if opts.include_extras { files.len() + 1 } else { files.len() } as u64;
        ProgressBar::new(len).with_style(
            ProgressStyle::with_template("{msg}\n[{elapsed_precise}] [{wide_bar:.cyan/blue}] [{pos}/{len}]")
                .unwrap()
                .progress_chars("#>-"),
        )
    };

    let path_already_exists = |p: &Path, indicator: &ProgressBar| {
        let message = format!("{} already exists", p.get_file_name());
        indicator.set_message(message);
        indicator.inc(1);
    };

    let skipping = |p: &Path, indicator: &ProgressBar, message: Option<&'static str>| {
        let message = format!("Skipping {} {}", p.get_file_name(), message.unwrap_or(""));
        indicator.set_message(message);
        indicator.inc(1);
    };

    let mut parent_set = HashSet::<PathBuf>::with_capacity(files.len() / 3);

    for file in files {
        let mut rel_path = file.strip_prefix(source_dir).unwrap().to_path_buf();

        let meta = get_track_data(&file, &file.get_file_ext())?;
        let is_syncable = opts.sync_codecs.contains(&meta.codec);
        let is_transcodable = !is_syncable && opts.transcode_codecs.contains(&meta.codec);

        // But why? Can't we use the check from codec.is_some()? No, not really.
        // We support syncing files that are part of the sync_extensions, so they don't go through the transcoding workflow.
        // So in cases like removing the temp file, it will remove the source file instead.
        let is_temp = opts.codec.is_some() && is_transcodable;
        let final_source_path: PathBuf;
        let final_target_path: PathBuf;

        if (is_temp || is_syncable)
            && let Some(x) = file.parent()
        {
            parent_set.insert(x.to_path_buf());
        }

        if is_transcodable && let Some(codec) = opts.codec {
            let new_ext = codec.extenstion_str();
            let bitrate = bitrate.expect("Bitrate must be set if codec is set");

            let target_rel = rel_path.with_extension(new_ext);
            let temp_path = temp_dir.join(&target_rel);
            let target_path = target_dir.join(&target_rel);

            if fs.exists(&target_path)? {
                path_already_exists(&target_rel, &indicator);
                continue;
            }

            let message = format!("Transcoding {} [{codec:?}@{bitrate}K]", rel_path.get_file_name());
            indicator.set_message(message);

            fs::create_dir_all(temp_path.parent().unwrap())?;
            transcode_file(&file, &temp_path, codec, bitrate)?;

            rel_path = target_rel;
            final_source_path = temp_path;
            final_target_path = target_path;
        } else if is_syncable {
            final_source_path = file;
            final_target_path = target_dir.join(&rel_path);

            if fs.exists(&final_target_path)? {
                path_already_exists(&rel_path, &indicator);
                continue;
            }
        } else {
            skipping(&rel_path, &indicator, Some("due to no codec"));
            continue;
        }

        indicator.set_message(format!("Syncing {:?}", rel_path.get_file_name()));
        if let Err(e) = fs.cp(&final_source_path, &final_target_path) {
            let context = format!("While copying {final_source_path:#?} to {final_target_path:#?}");
            return Err(e.with_context(context));
        }

        if is_temp {
            fs::remove_file(final_source_path)?;
        }

        indicator.inc(1);
    }

    if opts.include_extras {
        let exts = vec!["jpg", "png", "jpeg"];
        let files = read_selectively(&parent_set.into_iter().collect::<Vec<_>>(), &Some(exts))?;

        for file in files.into_iter().filter(|x| x.is_extra()) {
            let rel_path = file.strip_prefix(source_dir).unwrap();

            let message = format!("Syncing extra {}", rel_path.get_file_name());
            indicator.set_message(message);

            if fs.exists(&target_dir.join(rel_path))? {
                path_already_exists(rel_path, &indicator);
                continue;
            }

            indicator.set_message("Syncing extra files...");
            fs.cp(&file, &target_dir.join(rel_path))?;
        }

        indicator.inc(1);
    }

    indicator.finish_with_message("Done!");

    Ok(())
}
