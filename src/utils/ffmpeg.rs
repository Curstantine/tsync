use std::{path::Path, process::Command};

use crate::{
    errors::{Error, Result},
    format::Codec,
};

pub fn transcode_file<P: AsRef<Path>>(source: P, target: P, codec: Codec, bitrate: u32) -> Result<()> {
    let bitrate_str = format!("{bitrate}K");

    let output = match codec {
        Codec::Opus => Command::new("opusenc")
            .arg("--bitrate")
            .arg(&bitrate_str)
            .arg(source.as_ref())
            .arg(target.as_ref())
            .output(),
        _ => Command::new("ffmpeg")
            .arg("-i")
            .arg(source.as_ref())
            .arg("-c:a")
            .arg(codec.ffmpeg_lib())
            .arg("-b:a")
            .arg(&bitrate_str)
            .arg(target.as_ref())
            .output(),
    }?;

    if !output.status.success() {
        let message = format!("transcoder exited with code {}", output.status.code().unwrap_or(-1));
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Err(Error::descriptive(message));
    }

    Ok(())
}
