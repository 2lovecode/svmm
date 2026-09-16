use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::mods::manifest::{parse_manifest, ManifestDependency};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModDependency {
    pub unique_id: String,
    pub is_required: bool,
}

impl From<&ManifestDependency> for ModDependency {
    fn from(d: &ManifestDependency) -> Self {
        Self {
            unique_id: d.unique_id.clone(),
            is_required: d.is_required,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub folder_path: String,
    pub enabled: bool,
    pub minimum_api_version: Option<String>,
    pub update_keys: Vec<String>,
    pub dependencies: Vec<ModDependency>,
    pub status: String,
}

/// Build a relative path string under `mods_path` (prefer `/` separators for IPC stability).
pub fn relative_folder_path(mods_path: &Path, mod_dir: &Path) -> AppResult<String> {
    let rel = mod_dir.strip_prefix(mods_path).map_err(|_| {
        AppError::new("mod_path_invalid", "mod 目录不在 Mods 路径下")
            .with_detail(mod_dir.display().to_string())
    })?;
    Ok(rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/"))
}

fn leaf_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn is_mod_enabled(mod_dir: &Path) -> bool {
    !leaf_name(mod_dir).starts_with('.')
}

/// Collect every directory under `mods_path` that contains `manifest.json`.
fn collect_manifest_dirs(mods_path: &Path) -> AppResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !mods_path.is_dir() {
        return Err(AppError::new("mods_dir_missing", "未找到 Mods 目录")
            .with_detail(mods_path.display().to_string()));
    }
    walk_for_manifests(mods_path, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_for_manifests(dir: &Path, out: &mut Vec<PathBuf>) -> AppResult<()> {
    let entries = fs::read_dir(dir).map_err(|e| {
        AppError::new("mods_read_failed", "无法读取 Mods 目录").with_detail(e.to_string())
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| {
            AppError::new("mods_read_failed", "无法读取 Mods 目录项").with_detail(e.to_string())
        })?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Skip `.` / `..` only; dot-prefixed folders are disabled mods and must be scanned.
        if name_str == "." || name_str == ".." {
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        if path.join("manifest.json").is_file() {
            out.push(path.clone());
        }
        walk_for_manifests(&path, out)?;
    }
    Ok(())
}

pub fn entry_from_mod_dir(mods_path: &Path, mod_dir: &Path) -> AppResult<ModEntry> {
    let folder_path = relative_folder_path(mods_path, mod_dir)?;
    let enabled = is_mod_enabled(mod_dir);
    let folder = leaf_name(mod_dir);
    let display_name = folder.trim_start_matches('.').to_string();
    let manifest_path = mod_dir.join("manifest.json");

    if !manifest_path.is_file() {
        return Ok(ModEntry {
            id: folder_path.clone(),
            name: display_name,
            author: String::new(),
            version: String::new(),
            description: String::new(),
            folder_path,
            enabled,
            minimum_api_version: None,
            update_keys: Vec::new(),
            dependencies: Vec::new(),
            status: "missing_manifest".into(),
        });
    }

    let bytes = fs::read(&manifest_path).map_err(|e| {
        AppError::new("manifest_read_failed", "无法读取 manifest.json").with_detail(e.to_string())
    })?;

    match parse_manifest(&bytes) {
        Ok(m) => Ok(ModEntry {
            id: m.unique_id,
            name: m.name,
            author: m.author,
            version: m.version,
            description: m.description,
            folder_path,
            enabled,
            minimum_api_version: m.minimum_api_version,
            update_keys: m.update_keys,
            dependencies: m.dependencies.iter().map(ModDependency::from).collect(),
            status: "ok".into(),
        }),
        Err(_) => Ok(ModEntry {
            id: folder_path.clone(),
            name: display_name,
            author: String::new(),
            version: String::new(),
            description: String::new(),
            folder_path,
            enabled,
            minimum_api_version: None,
            update_keys: Vec::new(),
            dependencies: Vec::new(),
            status: "missing_manifest".into(),
        }),
    }
}

pub fn scan_mods(mods_path: &Path) -> AppResult<Vec<ModEntry>> {
    let dirs = collect_manifest_dirs(mods_path)?;
    let mut entries = Vec::with_capacity(dirs.len());
    for dir in dirs {
        entries.push(entry_from_mod_dir(mods_path, &dir)?);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::scan_mods;
    use crate::domain::mods::enable::set_mod_enabled;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_mods_fixture() -> PathBuf {
        let mods = std::env::temp_dir().join(format!(
            "svmm-mods-scan-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&mods);
        let mod_dir = mods.join("Ada.Test");
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(
            mod_dir.join("manifest.json"),
            br#"{
  "Name": "Test Mod",
  "Author": "Ada",
  "Version": "1.0.0",
  "Description": "Hi",
  "UniqueID": "Ada.Test"
}"#,
        )
        .unwrap();
        mods
    }

    #[test]
    fn scan_and_toggle_mod() {
        let mods = make_mods_fixture();
        let list = scan_mods(&mods).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].enabled);
        set_mod_enabled(&mods, &list[0].folder_path, false).unwrap();
        let list2 = scan_mods(&mods).unwrap();
        assert!(!list2[0].enabled);
        assert!(list2[0].folder_path.contains(".Ada") || list2[0].folder_path.starts_with('.'));
        let _ = fs::remove_dir_all(&mods);
    }
}
