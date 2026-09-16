use std::path::PathBuf;

/// Returns `%APPDATA%/svmm` on Windows (and the platform equivalent elsewhere).
pub fn app_data_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "svmm")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| {
            // Fallback if ProjectDirs cannot resolve (rare)
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join("svmm")
        })
}
