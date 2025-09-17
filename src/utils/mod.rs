use std::path::{Path, PathBuf};

use crate::errors::Result;

pub mod ffmpeg;
pub mod fs;
pub mod path;

pub fn parse_sync_list(source_dir: &Path, path: &Path) -> Result<Vec<PathBuf>> {
    let contents = std::fs::read_to_string(path)?;

    let splits = contents
        .split(&['\n', '\r'])
        .filter(|x| !x.is_empty() && !x.starts_with('#'))
        .map(|x| source_dir.join(x))
        .collect::<Vec<_>>();

    Ok(splits)
}
