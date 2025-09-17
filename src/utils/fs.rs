use std::{
    io::BufRead,
    path::{Path, PathBuf},
    process::Command,
};

use clap::ValueEnum;

use crate::errors::{Error, Result};

struct BackendADB;
struct BackendNone;

trait FSEmu {
    fn available() -> Result<bool>;
    fn cp(source: &Path, target: &Path) -> Result<()>;
    fn exists(source: &Path) -> Result<bool>;
}

#[derive(Debug, Clone, ValueEnum)]
pub enum FSBackend {
    /// Useful for android devices connected over tcpip or usb, and is recommended for all android-targeted syncs.
    Adb,

    /// Essentially the same as using none, but with validation for ftp addresses.
    Ftp,

    /// Not recommended for syncing between devices, but can be useful for moving files around on the same device.
    None,
}

impl FSBackend {
    pub fn available(&self) -> Result<bool> {
        match self {
            FSBackend::Adb => BackendADB::available(),
            FSBackend::Ftp => todo!("FTP backend not implemented"),
            FSBackend::None => BackendNone::available(),
        }
    }

    pub fn cp(&self, source: &Path, target: &Path) -> Result<()> {
        match self {
            FSBackend::Adb => BackendADB::cp(source, target),
            FSBackend::Ftp => todo!("FTP backend not implemented"),
            FSBackend::None => BackendNone::cp(source, target),
        }
    }

    pub fn exists(&self, source: &Path) -> Result<bool> {
        match self {
            FSBackend::Adb => BackendADB::exists(source),
            FSBackend::Ftp => todo!("FTP backend not implemented"),
            FSBackend::None => BackendNone::exists(source),
        }
    }
}

impl FSEmu for BackendNone {
    #[inline]
    fn available() -> Result<bool> {
        Ok(true)
    }

    fn cp(source: &Path, target: &Path) -> Result<()> {
        std::fs::copy(source, target)?;
        Ok(())
    }

    fn exists(source: &Path) -> Result<bool> {
        Ok(source.try_exists()?)
    }
}

impl FSEmu for BackendADB {
    fn available() -> Result<bool> {
        let is_adb_running = Command::new("adb")
            .arg("devices")
            .output()
            .map(|x| x.status.success() && x.stdout.lines().count() > 2)?;

        Ok(is_adb_running)
    }

    fn cp(source: &Path, target: &Path) -> Result<()> {
        let source = source.to_string_lossy().replace('\\', "/");
        let target = target.to_string_lossy().replace('\\', "/");

        let mut cmd = Command::new("adb");
        cmd.arg("push").arg(source).arg(target);

        let output = cmd.output()?;
        if !output.status.success() {
            let message = format!(
                "adb exited with code {} detailing {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8(output.stderr).unwrap()
            );
            return Err(Error::descriptive(message));
        }

        Ok(())
    }

    fn exists(source: &Path) -> Result<bool> {
        // For some reason adb shell only accepts "escaped paths", like path/dir/location.opus -> "path/dir/location" with string quotes
        let path = format!(r#""{}""#, source.to_string_lossy().replace('\\', "/"));
        let output = Command::new("adb").arg("shell").arg("ls").arg(path).output()?;

        Ok(output.status.success())
    }
}

pub fn read_dir_recursively<P: AsRef<Path>>(path: P, extensions: &Option<Vec<&'static str>>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::<PathBuf>::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let mut sub_files = read_dir_recursively(path, extensions)?;
            files.append(&mut sub_files);
            continue;
        }

        let ext = path.extension().and_then(|ext| ext.to_str()).unwrap();
        match extensions {
            Some(exts) if exts.contains(&ext) => files.push(path),
            None => files.push(path),
            _ => continue,
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
            continue;
        }

        let ext = path.extension().and_then(|ext| ext.to_str()).unwrap();
        match extensions {
            Some(exts) if exts.contains(&ext) => files.push(path.to_path_buf()),
            None => files.push(path.to_path_buf()),
            _ => continue,
        }
    }

    Ok(files)
}
