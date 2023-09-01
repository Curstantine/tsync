use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    errors::{Error, Result},
    format::CodecFormat,
};

pub fn is_adb_running() -> Result<bool> {
    let is_adb_running = Command::new("adb")
        .arg("devices")
        .output()
        .map(|output| output.status.success())?;
    Ok(is_adb_running)
}

pub fn transcode_file<P: AsRef<Path>>(source: P, target: P, codec: CodecFormat, bitrate: u32) -> Result<()> {
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
                .arg(codec.to_ffmpeg_lib())
                .arg("-b:a")
                .arg(format!("{}K", bitrate))
                .arg(target.as_ref().to_str().unwrap());

            cmd.output()
        }
    }?;

    if !output.status.success() {
        let message = format!("transcoder exited with code {}", output.status.code().unwrap_or(-1));
        return Err(Error::Descriptive(message));
    }

    Ok(())
}

pub fn push_to_adb_device<P: AsRef<Path>>(source: P, target: P) -> Result<()> {
    let mut cmd = Command::new("adb");
    cmd.arg("push")
        .arg(source.as_ref().to_str().unwrap())
        .arg(target.as_ref().to_str().unwrap());

    let output = cmd.output()?;
    if !output.status.success() {
        let message = format!("adb exited with code {}", output.status.code().unwrap_or(-1));
        return Err(Error::Descriptive(message));
    }

    Ok(())
}

pub fn read_dir_recursively<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let mut sub_files = read_dir_recursively(path)?;
            files.append(&mut sub_files);
        } else {
            files.push(path);
        }
    }

    Ok(files)
}

pub fn get_bitrate(codec: &CodecFormat, bitrate: &Option<u32>) -> Result<u32> {
    match bitrate {
        Some(bitrate) => {
            let (min, max) = match codec {
                CodecFormat::Opus => (6, 256),
                CodecFormat::Vorbis => (64, 500),
                CodecFormat::Mp3 => (32, 320),
            };

            if bitrate < &min || bitrate > &max {
                let message = format!("Bitrate must be between {} and {} for format {}", min, max, codec);
                return Err(Error::Descriptive(message));
            }

            Ok(*bitrate)
        }
        None => {
            let default = match codec {
                CodecFormat::Opus => 128,
                CodecFormat::Vorbis => 192,
                CodecFormat::Mp3 => 192,
            };

            Ok(default)
        }
    }
}
