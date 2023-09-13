use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    errors::{Error, Result},
    format::CodecFormat,
};

pub mod fs;

pub fn is_adb_running() -> Result<bool> {
    let is_adb_running = Command::new("adb")
        .arg("devices")
        .output()
        .map(|output| output.status.success())?;
    Ok(is_adb_running)
}

pub fn adb_file_exists<P: AsRef<Path>>(path: P) -> Result<bool> {
    let escaped_path = format!(r#""{}""#, path.as_ref().to_str().unwrap());
    let output = Command::new("adb").arg("shell").arg("ls").arg(escaped_path).output()?;

    Ok(output.status.success())
}

pub fn push_to_adb_device<P: AsRef<Path>>(source: P, target: P) -> Result<()> {
    let mut cmd = Command::new("adb");
    cmd.arg("push")
        .arg(source.as_ref().to_str().unwrap())
        .arg(target.as_ref().to_str().unwrap());

    let output = cmd.output()?;
    if !output.status.success() {
        let message = format!("adb exited with code {}", output.status.code().unwrap_or(-1));
        return Err(Error::descriptive(message));
    }

    Ok(())
}

pub fn transcode_file<P: AsRef<Path>>(source: P, target: P, codec: &CodecFormat, bitrate: u32) -> Result<()> {
    let output = match codec {
        CodecFormat::Opus => {
            let mut cmd = Command::new("opusenc");
            cmd.arg("--bitrate")
                .arg(format!("{}K", bitrate))
                .arg(source.as_ref().to_str().unwrap())
                .arg(target.as_ref().to_str().unwrap());

            cmd.output()
        }
        _ => {
            let mut cmd = Command::new("ffmpeg");
            cmd.arg("-i")
                .arg(source.as_ref().to_str().unwrap())
                .arg("-c:a")
                .arg(codec.get_ffmpeg_lib())
                .arg("-b:a")
                .arg(format!("{}K", bitrate))
                .arg(target.as_ref().to_str().unwrap());

            cmd.output()
        }
    }?;

    if !output.status.success() {
        let message = format!("transcoder exited with code {}", output.status.code().unwrap_or(-1));
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Err(Error::descriptive(message));
    }

    Ok(())
}

pub fn read_dir_recursively<P: AsRef<Path>>(path: P, extensions: &Option<Vec<String>>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let mut sub_files = read_dir_recursively(path, extensions)?;
            files.append(&mut sub_files);
        } else {
            let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or_default();

            match extensions {
                // wtf lol
                Some(exts) if exts.contains(&ext.to_string()) => files.push(path),
                None => files.push(path),
                _ => continue,
            }
        }
    }

    Ok(files)
}

pub fn split_optional_comma_string(s: Option<String>) -> Option<Vec<String>> {
    s.map(|exts| exts.split(',').map(|ext| ext.trim().to_string()).collect::<Vec<_>>())
}
