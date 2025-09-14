use std::path::{Path, PathBuf};

use clap::ValueEnum;

use crate::errors::{Error, Result};

#[derive(Debug, Clone, ValueEnum)]
pub enum FSBackend {
    /// Useful for android devices connected over tcpip or usb, and is recommended for all android-targeted syncs.
    Adb,

    /// Essentially the same as using none, but with validation for ftp addresses.
    Ftp,

    /// Not recommended for syncing between devices, but can be useful for moving files around on the same device.
    None,
}

pub fn get_file_name(p: &std::path::Path) -> String {
    p.file_name().unwrap().to_string_lossy().to_string()
}

pub fn get_file_ext(p: &std::path::Path) -> String {
    p.extension().unwrap().to_string_lossy().to_string()
}

pub fn read_dir_recursively<P: AsRef<Path>>(path: P, extensions: &Option<Vec<&'static str>>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::<PathBuf>::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let mut sub_files = read_dir_recursively(path, extensions)?;
            files.append(&mut sub_files);
        } else {
            let ext = path.extension().and_then(|ext| ext.to_str()).unwrap();
            match extensions {
                Some(exts) if exts.contains(&ext) => files.push(path),
                None => files.push(path),
                _ => continue,
            }
        }
    }

    Ok(files)
}

pub fn read_selectively<P: AsRef<Path>>(paths: &[P], extensions: &Option<Vec<&'static str>>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::<PathBuf>::new();

    for entry in paths {
        let path = entry.as_ref();

        if !path.exists() {
            return Err(Error::descriptive("File does not exist").with_context(path.to_string_lossy()));
        }

        if path.is_dir() {
            let mut sub_files = read_dir_recursively(path, extensions)?;
            files.append(&mut sub_files);
        } else {
            let ext = path.extension().and_then(|ext| ext.to_str()).unwrap();
            match extensions {
                Some(exts) if exts.contains(&ext) => files.push(path.to_path_buf()),
                None => files.push(path.to_path_buf()),
                _ => continue,
            }
        }
    }

    Ok(files)
}
