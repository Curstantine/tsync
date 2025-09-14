use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::errors::{Error, Result};

pub mod ffmpeg;
pub mod fs;

pub fn is_adb_running() -> Result<bool> {
    let is_adb_running = Command::new("adb")
        .arg("devices")
        .output()
        .map(|output| output.status.success())?;

    Ok(is_adb_running)
}

pub fn adb_file_exists(path: &Path) -> Result<bool> {
    // For some reason adb shell only accepts "escaped paths", like path/dir/location.opus -> "path/dir/location" with string quotes
    let path = format!(r#""{}""#, path.to_string_lossy().replace('\\', "/"));
    let output = Command::new("adb").arg("shell").arg("ls").arg(path).output()?;

    Ok(output.status.success())
}

pub fn push_to_adb_device(source: &Path, target: &Path) -> Result<()> {
    let source = source.to_string_lossy().replace('\\', "/");
    let target = target.to_string_lossy().replace('\\', "/");

    let mut cmd = Command::new("adb");
    cmd.arg("push").arg(source).arg(target);

    let output = cmd.output()?;
    if !output.status.success() {
        let message = format!("adb exited with code {}", output.status.code().unwrap_or(-1));
        return Err(Error::descriptive(message));
    }

    Ok(())
}

pub fn parse_sync_list(source_dir: &Path, path: &Path) -> Result<Vec<PathBuf>> {
    let contents = std::fs::read_to_string(path)?;

    let splits = contents
        .split(&['\n', '\r'])
        .filter(|x| !x.is_empty() && !x.starts_with('#'))
        .map(|x| source_dir.join(x))
        .collect::<Vec<_>>();

    Ok(splits)
}
