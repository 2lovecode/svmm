use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::storage::paths::app_data_dir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(alias = "game_path")]
    pub game_path: Option<PathBuf>,
    #[serde(alias = "smapi_path")]
    pub smapi_path: Option<PathBuf>,
    #[serde(alias = "mods_path")]
    pub mods_path: Option<PathBuf>,
    #[serde(alias = "last_profile_id")]
    pub last_profile_id: Option<String>,
    pub theme: String,
    pub language: String,
    #[serde(alias = "check_updates_on_startup")]
    pub check_updates_on_startup: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            game_path: None,
            smapi_path: None,
            mods_path: None,
            last_profile_id: None,
            theme: "system".into(),
            language: "zh-CN".into(),
            check_updates_on_startup: true,
        }
    }
}

fn settings_file_path() -> PathBuf {
    app_data_dir().join("settings.json")
}

pub fn load_settings_from(path: &Path) -> AppResult<Settings> {
    if !path.exists() {
        return Ok(Settings::default());
    }
    let text = fs::read_to_string(path).map_err(|e| {
        AppError::new("settings_read_failed", "无法读取设置文件").with_detail(e.to_string())
    })?;
    serde_json::from_str(&text).map_err(|e| {
        AppError::new("settings_parse_failed", "设置文件格式无效").with_detail(e.to_string())
    })
}

pub fn save_settings_to(path: &Path, settings: &Settings) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::new("settings_dir_failed", "无法创建设置目录").with_detail(e.to_string())
        })?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| {
        AppError::new("settings_serialize_failed", "无法序列化设置").with_detail(e.to_string())
    })?;
    fs::write(path, text).map_err(|e| {
        AppError::new("settings_write_failed", "无法写入设置文件").with_detail(e.to_string())
    })
}

pub fn load_settings() -> AppResult<Settings> {
    load_settings_from(&settings_file_path())
}

pub fn save_settings(settings: &Settings) -> AppResult<()> {
    save_settings_to(&settings_file_path(), settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn settings_round_trip_in_temp_dir() {
        let dir = std::env::temp_dir().join(format!("svmm-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let s = Settings {
            game_path: Some(r"C:\Games\Stardew Valley".into()),
            smapi_path: None,
            mods_path: None,
            last_profile_id: Some("default".into()),
            theme: "system".into(),
            language: "zh-CN".into(),
            check_updates_on_startup: true,
        };
        save_settings_to(&path, &s).unwrap();
        let loaded = load_settings_from(&path).unwrap();
        assert_eq!(loaded.game_path, s.game_path);
        assert_eq!(loaded.language, "zh-CN");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_loads_legacy_snake_case_keys() {
        let dir = std::env::temp_dir().join(format!(
            "svmm-test-snake-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        fs::write(
            &path,
            r#"{
  "game_path": "C:\\Games\\Stardew Valley",
  "smapi_path": null,
  "mods_path": null,
  "last_profile_id": "default",
  "theme": "system",
  "language": "zh-CN",
  "check_updates_on_startup": false
}"#,
        )
        .unwrap();
        let loaded = load_settings_from(&path).unwrap();
        assert_eq!(
            loaded.game_path.as_deref(),
            Some(std::path::Path::new(r"C:\Games\Stardew Valley"))
        );
        assert_eq!(loaded.last_profile_id.as_deref(), Some("default"));
        assert!(!loaded.check_updates_on_startup);
        let _ = fs::remove_dir_all(&dir);
    }
}
