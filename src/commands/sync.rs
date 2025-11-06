use std::{
    collections::HashSet,
    env, fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
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

struct FileMetadata {
    source: PathBuf,
    rel_path: PathBuf,
    codec: Codec,
}

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

    indicator.set_message("Analyzing files...");

    // Parallelize metadata reading for better performance
    let num_threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let file_metadata = {
        let (tx, rx) = mpsc::channel();
        let files_arc = Arc::new(files);
        let source_dir_arc = Arc::new(source_dir.to_path_buf());

        let chunk_size = (files_arc.len() / num_threads).max(1);

        for chunk_idx in 0..num_threads {
            let tx = tx.clone();
            let files = Arc::clone(&files_arc);
            let source_dir = Arc::clone(&source_dir_arc);

            thread::spawn(move || {
                let start = chunk_idx * chunk_size;
                let end = if chunk_idx == num_threads - 1 {
                    files.len()
                } else {
                    (start + chunk_size).min(files.len())
                };

                for i in start..end {
                    let file = &files[i];
                    let rel_path = file.strip_prefix(&*source_dir).unwrap().to_path_buf();
                    let file_ext = file.get_file_ext();

                    if let Ok(meta) = get_track_data(file, file_ext) {
                        let _ = tx.send(Some(FileMetadata {
                            source: file.clone(),
                            rel_path,
                            codec: meta.codec,
                        }));
                    } else {
                        let _ = tx.send(None);
                    }
                }
            });
        }

        drop(tx);

        let mut metadata = Vec::with_capacity(files_arc.len());
        for meta in rx.into_iter().flatten() {
            metadata.push(meta);
        }

        metadata
    };

    indicator.set_message("Building file list...");

    // Only build target file list if we actually need it (files exist)
    let target_file_list = if fs.exists(target_dir)? {
        fs.build_file_list(target_dir)?
    } else {
        HashSet::new()
    };

    let mut parent_set = HashSet::<PathBuf>::with_capacity(file_metadata.len() / 3);
    let mut transcode_jobs = Vec::with_capacity(file_metadata.len());
    let mut sync_jobs = Vec::with_capacity(file_metadata.len());
    let mut skipped = 0;

    for meta in file_metadata {
        let is_syncable = opts.sync_codecs.contains(&meta.codec);
        let is_transcodable = !is_syncable && opts.transcode_codecs.contains(&meta.codec);

        if let Some(x) = meta.source.parent()
            && (is_syncable || is_transcodable)
        {
            parent_set.insert(x.to_path_buf());
        }

        if is_transcodable && let Some(codec) = opts.codec {
            let new_ext = codec.extenstion_str();
            let target_rel = meta.rel_path.with_extension(new_ext);
            let target_path = target_dir.join(&target_rel);

            if target_file_list.contains(&target_path) {
                indicator.inc(1);
                continue;
            }

            transcode_jobs.push((meta.source, target_rel, meta.rel_path));
        } else if is_syncable {
            let target_path = target_dir.join(&meta.rel_path);

            if target_file_list.contains(&target_path) {
                indicator.inc(1);
                continue;
            }

            sync_jobs.push((meta.source, meta.rel_path));
        } else {
            skipped += 1;
            indicator.inc(1);
        }
    }

    if skipped > 0 {
        indicator.println(format!("Skipped {} files due to codec mismatch", skipped));
    }

    // Spawn transcoding threads with better load balancing
    let indicator = if !transcode_jobs.is_empty() {
        let codec = opts.codec.expect("Codec must be set if transcode jobs exist");
        let bitrate = bitrate.expect("Bitrate must be set if codec is set");
        let temp_dir = Arc::new(temp_dir);
        let indicator_arc = Arc::new(Mutex::new(indicator));

        let (tx, rx) = mpsc::channel();
        let chunk_size = (transcode_jobs.len() / num_threads).max(1);

        for chunk in transcode_jobs.chunks(chunk_size) {
            let tx = tx.clone();
            let chunk = chunk.to_vec();
            let temp_dir = Arc::clone(&temp_dir);
            let indicator_clone = Arc::clone(&indicator_arc);

            thread::spawn(move || {
                for (file, target_rel, rel_path) in chunk {
                    let temp_path = temp_dir.join(&target_rel);

                    if let Some(parent) = temp_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }

                    let result = transcode_file(&file, &temp_path, codec, bitrate)
                        .map(|_| (temp_path, target_rel, rel_path.clone()));

                    if result.is_ok()
                        && let Ok(indicator) = indicator_clone.lock()
                    {
                        indicator.inc(1);
                    }

                    let _ = tx.send(result);
                }
            });
        }

        drop(tx);

        // Collect results and perform sync
        let mut transcoded_files = Vec::with_capacity(transcode_jobs.len());
        for result in rx {
            if let Ok(data) = result {
                transcoded_files.push(data);
            } else if let Err(e) = result {
                return Err(e);
            }
        }

        // Get indicator back from Arc<Mutex>
        let indicator = Arc::try_unwrap(indicator_arc)
            .expect("All threads should be done")
            .into_inner()
            .expect("Mutex should not be poisoned");

        indicator.set_message("Syncing transcoded files...");

        for (temp_path, target_rel, _) in transcoded_files {
            let target_path = target_dir.join(&target_rel);

            if let Err(e) = fs.cp(&temp_path, &target_path) {
                let context = format!("While copying {temp_path:#?} to {target_path:#?}");
                return Err(e.with_context(context));
            }

            fs::remove_file(temp_path)?;
        }

        indicator
    } else {
        indicator
    };

    // Batch sync non-transcoded files for better performance
    indicator.set_message("Syncing files...");
    let sync_batch_size = 10;

    for chunk in sync_jobs.chunks(sync_batch_size) {
        for (file, rel_path) in chunk {
            let target_path = target_dir.join(rel_path);

            if let Err(e) = fs.cp(file, &target_path) {
                let context = format!("While copying {file:#?} to {target_path:#?}");
                return Err(e.with_context(context));
            }
        }
        indicator.inc(chunk.len() as u64);
    }

    if opts.include_extras {
        indicator.set_message("Syncing extra files...");

        let exts = vec!["jpg", "png", "jpeg"];
        let files = read_selectively(parent_set, &Some(exts))?;

        for file in files.into_iter().filter(|x| x.is_extra()) {
            let rel_path = file.strip_prefix(source_dir).unwrap();

            if fs.exists(&target_dir.join(rel_path))? {
                indicator.inc(1);
                continue;
            }

            fs.cp(&file, &target_dir.join(rel_path))?;
        }

        indicator.inc(1);
    }

    indicator.finish_with_message("Done!");

    Ok(())
}
