use clap::ValueEnum;

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
