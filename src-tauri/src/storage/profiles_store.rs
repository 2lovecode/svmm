use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::profiles::Profile;
use crate::error::{AppError, AppResult};
use crate::storage::paths::app_data_dir;

pub fn profiles_dir() -> PathBuf {
    app_data_dir().join("profiles")
}

fn profile_file_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}

pub fn ensure_profiles_dir_at(dir: &Path) -> AppResult<()> {
    fs::create_dir_all(dir).map_err(|e| {
        AppError::new("profiles_dir_failed", "无法创建 profiles 目录").with_detail(e.to_string())
    })
}

pub fn list_profiles_from(dir: &Path) -> AppResult<Vec<Profile>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(dir).map_err(|e| {
        AppError::new("profiles_read_failed", "无法读取 profiles 目录").with_detail(e.to_string())
    })?;
    let mut profiles = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            AppError::new("profiles_read_failed", "无法读取 profiles 目录")
                .with_detail(e.to_string())
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        profiles.push(load_profile_from(&path)?);
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    Ok(profiles)
}

pub fn load_profile_from(path: &Path) -> AppResult<Profile> {
    let text = fs::read_to_string(path).map_err(|e| {
        AppError::new("profile_read_failed", "无法读取 profile 文件").with_detail(e.to_string())
    })?;
    serde_json::from_str(&text).map_err(|e| {
        AppError::new("profile_parse_failed", "profile 文件格式无效").with_detail(e.to_string())
    })
}

pub fn load_profile_by_id_from(dir: &Path, id: &str) -> AppResult<Profile> {
    let path = profile_file_path(dir, id);
    if !path.exists() {
        return Err(
            AppError::new("profile_not_found", "未找到指定 profile").with_detail(id.to_string())
        );
    }
    load_profile_from(&path)
}

pub fn save_profile_to(dir: &Path, profile: &Profile) -> AppResult<()> {
    ensure_profiles_dir_at(dir)?;
    let path = profile_file_path(dir, &profile.id);
    let text = serde_json::to_string_pretty(profile).map_err(|e| {
        AppError::new("profile_serialize_failed", "无法序列化 profile").with_detail(e.to_string())
    })?;
    fs::write(&path, text).map_err(|e| {
        AppError::new("profile_write_failed", "无法写入 profile 文件").with_detail(e.to_string())
    })
}

pub fn delete_profile_from(dir: &Path, id: &str) -> AppResult<()> {
    let path = profile_file_path(dir, id);
    if !path.exists() {
        return Err(
            AppError::new("profile_not_found", "未找到指定 profile").with_detail(id.to_string())
        );
    }
    fs::remove_file(&path).map_err(|e| {
        AppError::new("profile_delete_failed", "无法删除 profile 文件").with_detail(e.to_string())
    })
}

pub fn load_profile_by_id(id: &str) -> AppResult<Profile> {
    load_profile_by_id_from(&profiles_dir(), id)
}

pub fn save_profile(profile: &Profile) -> AppResult<()> {
    save_profile_to(&profiles_dir(), profile)
}

pub fn delete_profile(id: &str) -> AppResult<()> {
    delete_profile_from(&profiles_dir(), id)
}

/// If no profiles exist, create a `default` group from `mod_ids`.
pub fn ensure_default_profile(
    dir: &Path,
    mod_ids: Vec<String>,
    now_iso: &str,
) -> AppResult<Vec<Profile>> {
    ensure_profiles_dir_at(dir)?;
    let existing = list_profiles_from(dir)?;
    if !existing.is_empty() {
        return Ok(existing);
    }
    let profile = Profile {
        id: "default".into(),
        name: "默认方案".into(),
        created_at: now_iso.to_string(),
        updated_at: now_iso.to_string(),
        mod_ids,
    };
    save_profile_to(dir, &profile)?;
    Ok(vec![profile])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "svmm-profiles-store-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ensure_default_creates_when_empty() {
        let dir = temp_dir();
        let list =
            ensure_default_profile(&dir, vec!["A".into(), "B".into()], "2026-01-01T00:00:00Z")
                .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "default");
        assert_eq!(list[0].mod_ids, vec!["A".to_string(), "B".to_string()]);
        let again = ensure_default_profile(&dir, vec!["C".into()], "2026-01-02T00:00:00Z").unwrap();
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].mod_ids, vec!["A".to_string(), "B".to_string()]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_load_round_trip() {
        let dir = temp_dir();
        let p = Profile {
            id: "abc".into(),
            name: "Demo".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            mod_ids: vec!["X".into()],
        };
        save_profile_to(&dir, &p).unwrap();
        let loaded = load_profile_by_id_from(&dir, "abc").unwrap();
        assert_eq!(loaded, p);
        let _ = fs::remove_dir_all(&dir);
    }
}
