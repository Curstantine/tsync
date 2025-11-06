use std::{
    collections::HashSet,
    env, fs, io,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    thread,
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

    indicator.set_message("Building file list...");

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
    let target_file_list = fs
        .exists(source_dir)?
        .then(|| fs.build_file_list(source_dir))
        .transpose()?
        .unwrap_or_else(|| HashSet::with_capacity(0));

    let mut transcode_jobs = Vec::new();
    let mut sync_jobs = Vec::new();

    for file in files {
        let rel_path = file.strip_prefix(source_dir).unwrap().to_path_buf();
        let meta = get_track_data(&file, &file.get_file_ext())?;
        let is_syncable = opts.sync_codecs.contains(&meta.codec);
        let is_transcodable = !is_syncable && opts.transcode_codecs.contains(&meta.codec);

        if let Some(x) = file.parent()
            && (is_syncable || is_transcodable)
        {
            parent_set.insert(x.to_path_buf());
        }

        if is_transcodable && let Some(codec) = opts.codec {
            let new_ext = codec.extenstion_str();
            let target_rel = rel_path.with_extension(new_ext);
            let target_path = target_dir.join(&target_rel);

            if target_file_list.contains(&target_path) {
                path_already_exists(&target_rel, &indicator);
                continue;
            }

            transcode_jobs.push((file, target_rel, rel_path));
        } else if is_syncable {
            let target_path = target_dir.join(&rel_path);

            if target_file_list.contains(&target_path) {
                path_already_exists(&rel_path, &indicator);
                continue;
            }

            sync_jobs.push((file, rel_path));
        } else {
            skipping(&rel_path, &indicator, Some("due to no codec"));
        }
    }

    // Spawn transcoding threads
    if !transcode_jobs.is_empty() {
        let num_threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let (tx, rx) = mpsc::channel();

        let codec = opts.codec;
        let bitrate = bitrate.expect("Bitrate must be set if codec is set");
        let temp_dir = Arc::new(temp_dir);

        for chunk in transcode_jobs.chunks((transcode_jobs.len() / num_threads).max(1)) {
            let tx = tx.clone();
            let chunk = chunk.to_vec();
            let temp_dir = Arc::clone(&temp_dir);
            let codec = codec.unwrap();

            thread::spawn(move || {
                for (file, target_rel, rel_path) in chunk {
                    let temp_path = temp_dir.join(&target_rel);

                    if let Some(parent) = temp_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }

                    let result =
                        transcode_file(&file, &temp_path, codec, bitrate).map(|_| (temp_path, target_rel, rel_path));

                    let _ = tx.send(result);
                }
            });
        }

        drop(tx);

        for result in rx {
            let (temp_path, target_rel, rel_path) = result?;

            indicator.set_message(format!("Transcoded {}", rel_path.get_file_name()));
            indicator.inc(1);

            let target_path = target_dir.join(&target_rel);
            indicator.set_message(format!("Syncing {:?}", target_rel.get_file_name()));

            if let Err(e) = fs.cp(&temp_path, &target_path) {
                let context = format!("While copying {temp_path:#?} to {target_path:#?}");
                return Err(e.with_context(context));
            }

            fs::remove_file(temp_path)?;
        }
    }

    // Sync non-transcoded files
    for (file, rel_path) in sync_jobs {
        let target_path = target_dir.join(&rel_path);

        indicator.set_message(format!("Syncing {:?}", rel_path.get_file_name()));
        if let Err(e) = fs.cp(&file, &target_path) {
            let context = format!("While copying {file:#?} to {target_path:#?}");
            return Err(e.with_context(context));
        }

        indicator.inc(1);
    }

    if opts.include_extras {
        let exts = vec!["jpg", "png", "jpeg"];
        let files = read_selectively(parent_set, &Some(exts))?;

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
