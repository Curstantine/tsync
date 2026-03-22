pub trait PathExtensions {
    fn get_file_name(&self) -> String;
    fn get_file_ext(&self) -> Option<String>;
    fn is_extra(&self) -> bool;
}

impl PathExtensions for std::path::Path {
    #[inline]
    fn get_file_name(&self) -> String {
        self.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| self.display().to_string())
    }

    #[inline]
    fn get_file_ext(&self) -> Option<String> {
        self.extension().map(|ext| ext.to_string_lossy().to_string())
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
