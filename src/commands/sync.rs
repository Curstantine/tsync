use std::{fs, io, path::Path};

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    errors::{self, Error},
    format::CodecFormat,
    utils::{is_adb_running, push_to_adb_device, read_dir_recursively, split_optional_comma_string, transcode_file},
};

const TEMP_DIR: &str = "./tmp";

pub fn run(
    source_dir: String,
    target_dir: String,
    codec: Option<String>,
    bitrate: Option<u32>,
    transcode_extensions: Option<String>,
    sync_extensions: Option<String>,
) -> errors::Result<()> {
    if !is_adb_running()? {
        let message = "adb is not running. Please start adb and try again.".to_string();
        return Err(Error::Descriptive(message));
    }

    match fs::create_dir(TEMP_DIR) {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_dir_all(TEMP_DIR)?;
            fs::create_dir(TEMP_DIR)?;
        }
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }

    let temp_dir = Path::new(TEMP_DIR);
    let source_dir = Path::new(&source_dir);
    let target_dir = Path::new(&target_dir);
    let codec = codec.map(CodecFormat::from_str).transpose()?;
    let bitrate = codec.as_ref().map(|c| c.get_matching_bitrate(bitrate)).transpose()?;
    let transcode_extensions = split_optional_comma_string(transcode_extensions).expect("Default supplied by clap");
    let sync_extensions = split_optional_comma_string(sync_extensions).expect("Default supplied by clap");

    let invalid_extensions = transcode_extensions
        .iter()
        .filter(|ext| sync_extensions.contains(ext))
        .map(|ext| ext.to_string())
        .collect::<Vec<_>>();

    if !invalid_extensions.is_empty() {
        let message = format!("Extensions cannot overlap: {}", invalid_extensions.join(","));
        return Err(Error::Descriptive(message));
    }

    // We can skip over the transcoding extensions if we don't have a codec.
    let readable_extensions = if codec.is_some() {
        transcode_extensions
            .iter()
            .chain(sync_extensions.iter())
            .map(|ext| ext.to_string())
            .collect::<Vec<_>>()
    } else {
        sync_extensions.iter().map(|ext| ext.to_string()).collect::<Vec<_>>()
    };

    let files = read_dir_recursively(source_dir, &Some(readable_extensions))?;
    println!("Found {} files", files.len().to_string().green());

    let indicator = ProgressBar::new(files.len() as u64);
    indicator.set_style(
        ProgressStyle::with_template("{msg}\n[{elapsed_precise}] [{wide_bar:.cyan/blue}] [{pos}/{len}]")
            .unwrap()
            .progress_chars("#>-"),
    );

    let get_file_name = |p: &Path| p.file_name().unwrap().to_str().unwrap().to_string();
    let get_extension = |p: &Path| p.extension().unwrap_or_default().to_str().unwrap().to_string();

    for file in files.into_iter() {
        let mut final_source_path = file.clone();
        let mut rel_path = file.strip_prefix(source_dir).unwrap().to_path_buf();
        let source_file_ext = get_extension(file.as_ref());

        // But why? Can't we use the check from codec.is_some()? No, not really.
        // We support syncing files that are part of the sync_extensions, so they don't go through the transcoding workflow.
        // So in cases like removing the temp file, it will remove the source file instead.
        let mut transcoded = false;

        match &codec {
            Some(codec) if transcode_extensions.contains(&source_file_ext) => {
                let new_ext = codec.get_extension_str();
                let temp_path = temp_dir.join(&rel_path).with_extension(new_ext);
                let bitrate = bitrate.expect("Bitrate must be set if codec is set");

                let message = format!("Transcoding {n} [{codec}@{bitrate}K]", n = get_file_name(&rel_path));
                indicator.set_message(message);

                fs::create_dir_all(temp_path.parent().unwrap())?;
                transcode_file(&file, &temp_path, codec, bitrate)?;

                transcoded = true;
                final_source_path = temp_path;
                rel_path.set_extension(new_ext);
            }
            // Ignore files with extensions that matches the sync extensions.
            Some(_) if sync_extensions.contains(&source_file_ext) => {}
            // Skip over files that don't match the sync extension when we don't have a codec.
            None if transcode_extensions.contains(&source_file_ext) => {
                let message = format!("Skipping {n}", n = get_file_name(&rel_path));
                indicator.set_message(message);
                indicator.inc(1);
                continue;
            }
            _ => unreachable!(),
        }

        indicator.set_message(format!("Pushing {n}", n = get_file_name(&rel_path)));
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
