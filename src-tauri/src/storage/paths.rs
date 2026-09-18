use std::path::PathBuf;

/// Windows: `%APPDATA%/svmm`. macOS: `~/Library/Application Support/svmm`.
pub fn app_data_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "svmm")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(fallback_app_data_dir)
}

pub fn library_dir() -> PathBuf {
    app_data_dir().join("library")
}

pub fn userdata_dir() -> PathBuf {
    app_data_dir().join("userdata")
}

pub fn apply_marker_path() -> PathBuf {
    app_data_dir().join("apply-pending.json")
}

fn fallback_app_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("svmm");
    }

    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("svmm")
}
