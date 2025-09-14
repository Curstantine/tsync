use std::{path::Path, process::Command};

use crate::{
    errors::{Error, Result},
    format::Codec,
};

pub fn strip_covers<P: AsRef<Path>>(source: P, parent: P) -> Result<()> {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i")
        .arg(source.as_ref())
        .arg("-map")
        .arg("0:v")
        .arg("-c")
        .arg("copy")
        .arg(parent.as_ref().join("cover_%d.jpg"));

    let output = cmd.output()?;

    if !output.status.success() {
        let message = format!("ffmpeg exited with code {}", output.status.code().unwrap_or(-1));
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Err(Error::descriptive(message));
    }

    Ok(())
}

pub fn transcode_file<P: AsRef<Path>>(source: P, target: P, codec: Codec, bitrate: u32) -> Result<()> {
    let output = match codec {
        Codec::Opus => {
            let mut cmd = Command::new("opusenc");
            cmd.arg("--bitrate")
                .arg(format!("{}K", bitrate))
                .arg(source.as_ref())
                .arg(target.as_ref());

            cmd.output()
        }
        _ => {
            let mut cmd = Command::new("ffmpeg");
            cmd.arg("-i")
                .arg(source.as_ref())
                .arg("-c:a")
                .arg(codec.get_ffmpeg_lib())
                .arg("-b:a")
                .arg(format!("{}K", bitrate))
                .arg(target.as_ref());

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
