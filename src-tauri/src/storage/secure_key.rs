use keyring::Entry;

use crate::error::{AppError, AppResult};

const SERVICE: &str = "svmm";
const ACCOUNT: &str = "nexus_api_key";

fn entry() -> AppResult<Entry> {
    Entry::new(SERVICE, ACCOUNT).map_err(|_| {
        AppError::new("nexus_keyring_failed", "无法访问系统凭据存储")
    })
}

pub fn set_nexus_api_key(key: &str) -> AppResult<()> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("nexus_key_empty", "API 密钥不能为空"));
    }
    entry()?
        .set_password(trimmed)
        .map_err(|_| AppError::new("nexus_key_store_failed", "无法保存 Nexus API 密钥"))
}

pub fn clear_nexus_api_key() -> AppResult<()> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(AppError::new(
            "nexus_key_clear_failed",
            "无法清除 Nexus API 密钥",
        )),
    }
}

pub fn has_nexus_api_key() -> bool {
    match entry() {
        Ok(e) => e.get_password().map(|p| !p.trim().is_empty()).unwrap_or(false),
        Err(_) => false,
    }
}

pub fn get_nexus_api_key() -> AppResult<String> {
    let password = entry()?
        .get_password()
        .map_err(|e| match e {
            keyring::Error::NoEntry => {
                AppError::new("nexus_key_missing", "尚未配置 Nexus API 密钥")
            }
            _ => AppError::new("nexus_key_read_failed", "无法读取 Nexus API 密钥"),
        })?;
    let trimmed = password.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "nexus_key_missing",
            "尚未配置 Nexus API 密钥",
        ));
    }
    Ok(trimmed.to_string())
}
