use std::{fs, io, path::Path};

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    errors::{self, Error},
    format::CodecFormat,
    utils::{get_bitrate, is_adb_running, push_to_adb_device, read_dir_recursively, transcode_file},
};

const TEMP_DIR: &str = "./tmp";

pub fn sync<P: AsRef<Path>>(
    source_dir: P,
    target_dir: P,
    format: Option<String>,
    bitrate: Option<u32>,
    filter_extensions: Option<String>,
) -> errors::Result<()> {
    let source_dir = source_dir.as_ref();
    let target_dir = target_dir.as_ref();
    let temp_dir = Path::new(TEMP_DIR);

    if !is_adb_running()? {
        let message = "adb is not running. Please start adb and try again.".to_string();
        return Err(Error::Descriptive(message));
    }

    match fs::create_dir(TEMP_DIR) {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => fs::remove_dir_all(TEMP_DIR)?,
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }

    let filter_extensions = filter_extensions
        .map(|exts| exts.split(',').map(|ext| ext.trim().to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
    let files = read_dir_recursively(source_dir)?
        .into_iter()
        .filter(|f| {
            if filter_extensions.is_empty() {
                return true;
            }

            let ext = f.extension().unwrap_or_default().to_str().unwrap_or_default();
            filter_extensions.contains(&ext.to_string())
        })
        .collect::<Vec<_>>();

    println!("Found {} files", files.len().to_string().green());

    let indicator = ProgressBar::new(files.len() as u64);
    indicator.set_style(
        ProgressStyle::with_template("{msg}\n[{elapsed_precise}] [{wide_bar:.cyan/blue}] [{pos}/{len}]")
            .unwrap()
            .progress_chars("#>-"),
    );

    for file in files.into_iter() {
        let mut final_source_path = file.clone();

        // Path relative to both source and temp dir to this file.
        let rel_path = file.strip_prefix(source_dir).unwrap();
        let get_file_name = || rel_path.file_name().unwrap().to_str().unwrap().to_string();

        if let Some(format) = &format {
            let temp_path = temp_dir.join(rel_path).with_extension(format);

            let codec = CodecFormat::from_str(format)?;
            let bitrate = get_bitrate(&codec, &bitrate)?;

            let message = format!(
                "Transcoding {file_name} as {format} ({bitrate}K)",
                file_name = get_file_name(),
            );
            indicator.set_message(message);
            fs::create_dir_all(temp_path.parent().unwrap())?;
            transcode_file(&file, &temp_path, codec, bitrate)?;

            final_source_path = temp_path;
        }

        let message = format!("Pushing {file_name}", file_name = get_file_name());
        indicator.set_message(message);

        let target_path = target_dir.join(rel_path.with_extension("opus"));
        push_to_adb_device(&final_source_path, &target_path)?;

        // Marked as a temp file if it was transcoded.
        if format.is_some() {
            fs::remove_file(final_source_path)?;
        }

        indicator.inc(1);
    }

    indicator.finish();

    Ok(())
}
