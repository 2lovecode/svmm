use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::storage::paths::app_data_dir;

fn secret_file() -> PathBuf {
    app_data_dir().join("secrets").join("nexus_api_key")
}

pub fn set_nexus_api_key(key: &str) -> AppResult<()> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("nexus_key_empty", "API 密钥不能为空"));
    }
    write_file_secret(&secret_file(), trimmed)
}

pub fn clear_nexus_api_key() -> AppResult<()> {
    delete_file_secret(&secret_file())?;
    if has_nexus_api_key() {
        return Err(AppError::new(
            "nexus_key_clear_failed",
            "无法清除 Nexus API 密钥",
        ));
    }
    Ok(())
}

pub fn has_nexus_api_key() -> bool {
    get_nexus_api_key().is_ok()
}

pub fn get_nexus_api_key() -> AppResult<String> {
    if let Some(key) = nonempty(read_file_secret(&secret_file())?) {
        return Ok(key);
    }
    Err(AppError::new(
        "nexus_key_missing",
        "尚未配置 Nexus API 密钥",
    ))
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn write_file_secret(path: &Path, key: &str) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::new("nexus_key_store_failed", "无法保存 Nexus API 密钥")
                .with_detail(e.to_string())
        })?;
        restrict_dir(parent);
    }
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| {
        AppError::new("nexus_key_store_failed", "无法保存 Nexus API 密钥")
            .with_detail(e.to_string())
    })?;
    file.write_all(key.as_bytes()).map_err(|e| {
        AppError::new("nexus_key_store_failed", "无法保存 Nexus API 密钥")
            .with_detail(e.to_string())
    })?;
    restrict_file(path)?;
    Ok(())
}

fn read_file_secret(path: &Path) -> AppResult<Option<String>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|e| {
        AppError::new("nexus_key_read_failed", "无法读取 Nexus API 密钥").with_detail(e.to_string())
    })?;
    Ok(Some(text))
}

fn delete_file_secret(path: &Path) -> AppResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(
            AppError::new("nexus_key_clear_failed", "无法清除 Nexus API 密钥")
                .with_detail(err.to_string()),
        ),
    }
}

fn restrict_dir(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn restrict_file(path: &Path) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| {
            AppError::new("nexus_key_store_failed", "无法保存 Nexus API 密钥")
                .with_detail(e.to_string())
        })?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_secret() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "svmm-secret-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir.join("nexus_api_key")
    }

    #[test]
    fn file_secret_round_trip() {
        let path = temp_secret();
        write_file_secret(&path, "secret-key").unwrap();
        assert_eq!(
            read_file_secret(&path).unwrap().as_deref(),
            Some("secret-key")
        );
        delete_file_secret(&path).unwrap();
        assert_eq!(read_file_secret(&path).unwrap(), None);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn delete_missing_file_is_ok() {
        let path = temp_secret();
        delete_file_secret(&path).unwrap();
    }
}
