use std::ffi::OsStr;

pub trait PathExtensions {
    fn get_file_name(&self) -> String;
    fn get_file_ext(&self) -> &str;
    fn is_extra(&self) -> bool;
}

impl PathExtensions for std::path::Path {
    #[inline]
    fn get_file_name(&self) -> String {
        self.file_name().unwrap().to_string_lossy().to_string()
    }

    #[inline]
    fn get_file_ext(&self) -> &str {
        self.extension().and_then(OsStr::to_str).unwrap_or("")
    }

    #[inline]
    fn is_extra(&self) -> bool {
        self.file_name().is_some_and(|e| {
            matches!(
                e.to_string_lossy().to_lowercase().as_str(),
                "cover.jpg" | "cover.png" | "folder.jpg" | "folder.png" | "front.jpg" | "front.png"
            )
        })
    }
}
