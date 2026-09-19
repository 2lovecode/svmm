//! Local mod library, shared userdata, and profile deployment.
//!
//! Packages live under the library root, never in the game `Mods/` folder,
//! until a profile is applied or a deployed copy is synced after a version swap.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::mods::install::safe_extract_zip;
use crate::domain::mods::manifest::parse_manifest;
use crate::domain::mods::scan::{self, is_svmm_backup_dir_name, is_svmm_extract_dir_name};
use crate::domain::profiles::{ApplyReport, Profile};
use crate::error::{AppError, AppResult};

pub const PACKAGE_FILE: &str = ".svmm-package.json";

const SMAPI_FOLDERS: &[&str] = &["ConsoleCommands", "ErrorHandler", "SaveBackup"];
const SMAPI_IDS: &[&str] = &[
    "SMAPI.ConsoleCommands",
    "SMAPI.ErrorHandler",
    "SMAPI.SaveBackup",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackageFile {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMod {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    /// False when the id is a folder name because the package had no UniqueID.
    pub id_from_manifest: bool,
    pub category: Option<String>,
    pub nexus_mod_id: Option<u32>,
    pub nexus_file_id: Option<u64>,
    pub folder_name: String,
    pub files: Vec<PackageFile>,
}

#[derive(Debug, Clone, Default)]
pub struct ImportMeta {
    pub category: Option<String>,
    pub nexus_mod_id: Option<u32>,
    pub nexus_file_id: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct LibraryQuery<'a> {
    pub category: Option<&'a str>,
    pub id: Option<&'a str>,
    pub keyword: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyMarker {
    mods_path: String,
    backup_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileState {
    pub profile: Profile,
    pub mods: Vec<LibraryMod>,
    pub applied: bool,
}

pub fn is_smapi_bundled(folder_name: &str, unique_id: Option<&str>) -> bool {
    let folder = folder_name.trim().trim_start_matches('.');
    if SMAPI_FOLDERS
        .iter()
        .any(|name| folder.eq_ignore_ascii_case(name))
    {
        return true;
    }
    unique_id.is_some_and(|id| SMAPI_IDS.iter().any(|known| id == *known))
}

pub fn cmp_version(local: &str, remote: &str) -> std::cmp::Ordering {
    fn parts(value: &str) -> Option<Vec<u64>> {
        if value.is_empty() {
            return None;
        }
        value
            .split(|c| c == '.' || c == '-' || c == '_')
            .map(|part| part.parse::<u64>().ok())
            .collect()
    }
    match (parts(local), parts(remote)) {
        (Some(a), Some(b)) => {
            let n = a.len().max(b.len());
            for i in 0..n {
                let av = a.get(i).copied().unwrap_or(0);
                let bv = b.get(i).copied().unwrap_or(0);
                match av.cmp(&bv) {
                    std::cmp::Ordering::Equal => {}
                    other => return other,
                }
            }
            std::cmp::Ordering::Equal
        }
        _ => local.cmp(remote),
    }
}

/// `missing` | `owned` | `update`. Compared against the library, not `Mods/`.
pub fn nexus_library_status(
    mods: &[LibraryMod],
    nexus_mod_id: u32,
    remote_version: &str,
) -> &'static str {
    let Some(found) = mods.iter().find(|m| m.nexus_mod_id == Some(nexus_mod_id)) else {
        return "missing";
    };
    if cmp_version(&found.version, remote_version).is_ge() {
        "owned"
    } else {
        "update"
    }
}

fn io_err(code: &str, message: &str, err: impl std::fmt::Display) -> AppError {
    AppError::new(code, message).with_detail(err.to_string())
}

fn safe_component(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_start_matches('.');
    if trimmed.is_empty() {
        "mod".to_string()
    } else {
        trimmed.chars().take(80).collect()
    }
}

fn package_path(library_root: &Path, id: &str) -> PathBuf {
    library_root.join(safe_component(id)).join(PACKAGE_FILE)
}

fn mod_dir(library_root: &Path, id: &str) -> PathBuf {
    library_root.join(safe_component(id))
}

fn dismissed_path(library_root: &Path) -> PathBuf {
    library_root.join(".dismissed-ids.json")
}

fn read_dismissed(library_root: &Path) -> AppResult<HashSet<String>> {
    let path = dismissed_path(library_root);
    if !path.is_file() {
        return Ok(HashSet::new());
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| io_err("library_read_failed", "无法读取已删除记录", e))?;
    let ids: Vec<String> = serde_json::from_str(&text).map_err(|e| {
        AppError::new("library_parse_failed", "已删除记录无效").with_detail(e.to_string())
    })?;
    Ok(ids.into_iter().collect())
}

fn write_dismissed(library_root: &Path, ids: &HashSet<String>) -> AppResult<()> {
    fs::create_dir_all(library_root)
        .map_err(|e| io_err("library_write_failed", "无法创建本地库", e))?;
    let mut list: Vec<String> = ids.iter().cloned().collect();
    list.sort();
    let text = serde_json::to_string_pretty(&list).map_err(|e| {
        AppError::new("library_serialize_failed", "无法写入已删除记录").with_detail(e.to_string())
    })?;
    fs::write(dismissed_path(library_root), text)
        .map_err(|e| io_err("library_write_failed", "无法写入已删除记录", e))
}

fn is_dismissed(ids: &HashSet<String>, id: &str) -> bool {
    ids.iter().any(|item| item.eq_ignore_ascii_case(id))
}

fn remember_dismissed(library_root: &Path, id: &str) -> AppResult<()> {
    let mut ids = read_dismissed(library_root)?;
    if is_dismissed(&ids, id) {
        return Ok(());
    }
    ids.insert(id.to_string());
    write_dismissed(library_root, &ids)
}

fn forget_dismissed(library_root: &Path, id: &str) -> AppResult<()> {
    let mut ids = read_dismissed(library_root)?;
    let before = ids.len();
    ids.retain(|item| !item.eq_ignore_ascii_case(id));
    if ids.len() == before {
        return Ok(());
    }
    write_dismissed(library_root, &ids)
}

fn remove_id_from_profiles(profiles_dir: &Path, id: &str) -> AppResult<()> {
    if !profiles_dir.exists() {
        return Ok(());
    }
    let profiles = crate::storage::profiles_store::list_profiles_from(profiles_dir)?;
    for mut profile in profiles {
        let before = profile.mod_ids.len();
        profile
            .mod_ids
            .retain(|item| !item.eq_ignore_ascii_case(id));
        if profile.mod_ids.len() != before {
            crate::storage::profiles_store::save_profile_to(profiles_dir, &profile)?;
        }
    }
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_file(path: &Path) -> AppResult<String> {
    let bytes = fs::read(path).map_err(|e| io_err("library_read_failed", "无法读取模组文件", e))?;
    Ok(sha256_bytes(&bytes))
}

fn rel_key(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn is_internal_file(rel: &str) -> bool {
    rel == PACKAGE_FILE
        || rel.split('/').any(|part| {
            part.starts_with(".svmm-")
                || is_svmm_backup_dir_name(part)
                || is_svmm_extract_dir_name(part)
        })
}

fn walk_files(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) -> AppResult<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries =
        fs::read_dir(dir).map_err(|e| io_err("library_read_failed", "无法读取目录", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| io_err("library_read_failed", "无法读取目录项", e))?;
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap_or(&path);
        if is_internal_file(&rel_key(rel)) {
            continue;
        }
        if path.is_dir() {
            walk_files(&path, base, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

fn copy_dir_filtered(src: &Path, dst: &Path) -> AppResult<()> {
    fs::create_dir_all(dst).map_err(|e| io_err("library_copy_failed", "无法创建目录", e))?;
    let mut files = Vec::new();
    walk_files(src, src, &mut files)?;
    for file in files {
        let rel = file.strip_prefix(src).unwrap_or(&file);
        let target = dst.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| io_err("library_copy_failed", "无法创建目录", e))?;
        }
        fs::copy(&file, &target)
            .map_err(|e| io_err("library_copy_failed", "无法复制模组文件", e))?;
    }
    Ok(())
}

fn remove_dir_if_exists(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|e| io_err("library_delete_failed", "无法删除目录", e))?;
    }
    Ok(())
}

fn nexus_id_from_keys(keys: &[String]) -> Option<u32> {
    for key in keys {
        let rest = key
            .trim()
            .strip_prefix("Nexus:")
            .or_else(|| key.trim().strip_prefix("nexus:"))?;
        if let Ok(id) = rest.trim().parse::<u32>() {
            return Some(id);
        }
    }
    None
}

pub fn load_mod(library_root: &Path, id: &str) -> AppResult<LibraryMod> {
    let path = package_path(library_root, id);
    if !path.is_file() {
        return Err(AppError::new("library_mod_missing", "本地库中没有这个模组")
            .with_detail(id.to_string()));
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| io_err("library_read_failed", "无法读取本地库记录", e))?;
    serde_json::from_str(&text).map_err(|e| {
        AppError::new("library_parse_failed", "本地库记录无效").with_detail(e.to_string())
    })
}

pub fn list_mods(library_root: &Path, query: LibraryQuery<'_>) -> AppResult<Vec<LibraryMod>> {
    if !library_root.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(library_root)
        .map_err(|e| io_err("library_read_failed", "无法读取本地库", e))?;
    let mut mods = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| io_err("library_read_failed", "无法读取本地库", e))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        let record = path.join(PACKAGE_FILE);
        if !record.is_file() {
            continue;
        }
        let text = fs::read_to_string(&record)
            .map_err(|e| io_err("library_read_failed", "无法读取本地库记录", e))?;
        let parsed: LibraryMod = serde_json::from_str(&text).map_err(|e| {
            AppError::new("library_parse_failed", "本地库记录无效").with_detail(e.to_string())
        })?;
        if !matches_query(&parsed, &query) {
            continue;
        }
        mods.push(parsed);
    }
    mods.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    Ok(mods)
}

fn matches_query(mod_entry: &LibraryMod, query: &LibraryQuery<'_>) -> bool {
    if let Some(category) = query.category.map(str::trim).filter(|s| !s.is_empty()) {
        if mod_entry.category.as_deref() != Some(category) {
            return false;
        }
    }
    if let Some(id) = query.id.map(str::trim).filter(|s| !s.is_empty()) {
        if mod_entry.id != id {
            return false;
        }
    }
    if let Some(keyword) = query.keyword.map(str::trim).filter(|s| !s.is_empty()) {
        let kw = keyword.to_lowercase();
        let hay = format!(
            "{} {} {}",
            mod_entry.name.to_lowercase(),
            mod_entry.author.to_lowercase(),
            mod_entry.id.to_lowercase()
        );
        if !hay.contains(&kw) {
            return false;
        }
    }
    true
}

pub fn import_mod_dir(
    library_root: &Path,
    source: &Path,
    meta: ImportMeta,
) -> AppResult<LibraryMod> {
    if !source.is_dir() {
        return Err(
            AppError::new("library_import_failed", "找不到要入库的模组目录")
                .with_detail(source.display().to_string()),
        );
    }
    let manifest_bytes = fs::read(source.join("manifest.json")).map_err(|e| {
        AppError::new("manifest_missing", "压缩包里没有 manifest.json").with_detail(e.to_string())
    })?;
    let manifest = parse_manifest(&manifest_bytes)?;
    let folder_raw = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ImportedMod".to_string());
    if is_smapi_bundled(&folder_raw, Some(&manifest.unique_id)) {
        return Err(AppError::new("smapi_bundled", "SMAPI 自带模组不进入本地库"));
    }
    let id_from_manifest = !manifest.unique_id.trim().is_empty();
    let id = if id_from_manifest {
        manifest.unique_id.clone()
    } else {
        safe_component(&folder_raw)
    };
    let folder_name = safe_component(folder_raw.trim_start_matches('.'));
    fs::create_dir_all(library_root)
        .map_err(|e| io_err("library_write_failed", "无法创建本地库", e))?;
    let dest = mod_dir(library_root, &id);
    remove_dir_if_exists(&dest)?;
    copy_dir_filtered(source, &dest)?;

    let mut files_on_disk = Vec::new();
    walk_files(&dest, &dest, &mut files_on_disk)?;
    let mut files = Vec::new();
    for file in files_on_disk {
        let rel = file.strip_prefix(&dest).unwrap_or(&file);
        let key = rel_key(rel);
        files.push(PackageFile {
            sha256: sha256_file(&file)?,
            path: key,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let nexus_mod_id = meta
        .nexus_mod_id
        .or_else(|| nexus_id_from_keys(&manifest.update_keys));
    let record = LibraryMod {
        id,
        name: manifest.name,
        author: manifest.author,
        version: manifest.version,
        description: manifest.description,
        id_from_manifest,
        category: meta.category.filter(|c| !c.trim().is_empty()),
        nexus_mod_id,
        nexus_file_id: meta.nexus_file_id,
        folder_name,
        files,
    };
    let text = serde_json::to_string_pretty(&record).map_err(|e| {
        AppError::new("library_serialize_failed", "无法写入本地库记录").with_detail(e.to_string())
    })?;
    fs::write(dest.join(PACKAGE_FILE), text)
        .map_err(|e| io_err("library_write_failed", "无法写入本地库记录", e))?;
    forget_dismissed(library_root, &record.id)?;
    Ok(record)
}

pub fn import_zip(zip_path: &Path, library_root: &Path, meta: ImportMeta) -> AppResult<LibraryMod> {
    let staging = std::env::temp_dir().join(format!("svmm-extract-{}", uuid::Uuid::new_v4()));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)
        .map_err(|e| io_err("library_import_failed", "无法创建临时目录", e))?;
    let extracted = match safe_extract_zip(zip_path, &staging) {
        Ok(path) => path,
        Err(err) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(err);
        }
    };
    let result = ingest_extracted(&extracted, library_root, None, None, meta);
    let _ = fs::remove_dir_all(&staging);
    result?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::new("zip_no_mod", "压缩包中未找到有效的模组目录"))
}

pub fn replace_from_zip(
    library_root: &Path,
    userdata_root: &Path,
    mods_path: Option<&Path>,
    zip_path: &Path,
    meta: ImportMeta,
) -> AppResult<Vec<LibraryMod>> {
    let staging = std::env::temp_dir().join(format!("svmm-extract-{}", uuid::Uuid::new_v4()));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)
        .map_err(|e| io_err("library_import_failed", "无法创建临时目录", e))?;
    let extracted = match safe_extract_zip(zip_path, &staging) {
        Ok(path) => path,
        Err(err) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(err);
        }
    };
    let result = ingest_extracted(
        &extracted,
        library_root,
        Some(userdata_root),
        mods_path,
        meta,
    );
    let _ = fs::remove_dir_all(&staging);
    result
}

fn ingest_extracted(
    extracted: &[PathBuf],
    library_root: &Path,
    userdata_root: Option<&Path>,
    mods_path: Option<&Path>,
    meta: ImportMeta,
) -> AppResult<Vec<LibraryMod>> {
    let mut imported = Vec::new();
    for path in extracted {
        let result = if let Some(userdata) = userdata_root {
            replace_package(
                library_root,
                userdata,
                mods_path,
                path,
                meta.clone(),
                false,
            )
        } else {
            import_mod_dir(library_root, path, meta.clone())
        };
        match result {
            Ok(item) => imported.push(item),
            Err(err) if err.code == "smapi_bundled" => {}
            Err(err) => return Err(err),
        }
    }
    if imported.is_empty() {
        return Err(AppError::new(
            "zip_no_mod",
            "压缩包中未找到有效的模组目录",
        ));
    }
    Ok(imported)
}

pub fn delete_mod(library_root: &Path, profiles_dir: &Path, id: &str) -> AppResult<()> {
    let dir = mod_dir(library_root, id);
    if !dir.join(PACKAGE_FILE).is_file() {
        return Err(AppError::new("library_mod_missing", "本地库中没有这个模组")
            .with_detail(id.to_string()));
    }
    remember_dismissed(library_root, id)?;
    remove_id_from_profiles(profiles_dir, id)?;
    remove_dir_if_exists(&dir)?;
    Ok(())
}

pub fn capture_userdata(
    userdata_root: &Path,
    id: &str,
    deployed_dir: &Path,
    package_files: &[PackageFile],
) -> AppResult<()> {
    if !deployed_dir.is_dir() {
        return Ok(());
    }
    let listed: HashMap<&str, &str> = package_files
        .iter()
        .map(|f| (f.path.as_str(), f.sha256.as_str()))
        .collect();
    let mut files = Vec::new();
    walk_files(deployed_dir, deployed_dir, &mut files)?;
    let dest_root = userdata_root.join(safe_component(id));
    for file in files {
        let rel = file.strip_prefix(deployed_dir).unwrap_or(&file);
        let key = rel_key(rel);
        if is_internal_file(&key) {
            continue;
        }
        let hash = sha256_file(&file)?;
        let differs = match listed.get(key.as_str()) {
            None => true,
            Some(expected) => *expected != hash,
        };
        if !differs {
            continue;
        }
        let target = dest_root.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| io_err("userdata_write_failed", "无法保存玩家数据", e))?;
        }
        fs::copy(&file, &target)
            .map_err(|e| io_err("userdata_write_failed", "无法保存玩家数据", e))?;
    }
    Ok(())
}

pub fn restore_userdata(userdata_root: &Path, id: &str, deployed_dir: &Path) -> AppResult<()> {
    let src = userdata_root.join(safe_component(id));
    if !src.is_dir() {
        return Ok(());
    }
    copy_dir_filtered(&src, deployed_dir)
}

fn find_deployed_dir(mods_path: &Path, id: &str) -> AppResult<Option<PathBuf>> {
    if !mods_path.is_dir() {
        return Ok(None);
    }
    let entries = scan::scan_mods(mods_path)?;
    Ok(entries.into_iter().find(|e| e.id == id).map(|e| {
        let mut path = mods_path.to_path_buf();
        for part in e.folder_path.split(['/', '\\']) {
            if !part.is_empty() && part != "." && part != ".." {
                path.push(part);
            }
        }
        path
    }))
}

/// Replace the single current package. If that mod is deployed, sync it and roll both back on failure.
pub fn replace_package(
    library_root: &Path,
    userdata_root: &Path,
    mods_path: Option<&Path>,
    source: &Path,
    meta: ImportMeta,
    fail_after_deploy_delete: bool,
) -> AppResult<LibraryMod> {
    let preview = fs::read(source.join("manifest.json")).map_err(|e| {
        AppError::new("manifest_missing", "压缩包里没有 manifest.json").with_detail(e.to_string())
    })?;
    let manifest = parse_manifest(&preview)?;
    let id = if manifest.unique_id.trim().is_empty() {
        safe_component(
            &source
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        )
    } else {
        manifest.unique_id.clone()
    };

    let old = load_mod(library_root, &id).ok();
    let old_files = old.as_ref().map(|m| m.files.clone()).unwrap_or_default();
    let library_backup =
        std::env::temp_dir().join(format!("svmm-lib-backup-{}", uuid::Uuid::new_v4()));
    let had_library = mod_dir(library_root, &id).is_dir();
    if had_library {
        copy_dir_filtered(&mod_dir(library_root, &id), &library_backup)?;
        if mod_dir(library_root, &id).join(PACKAGE_FILE).is_file() {
            fs::copy(
                mod_dir(library_root, &id).join(PACKAGE_FILE),
                library_backup.join(PACKAGE_FILE),
            )
            .map_err(|e| io_err("library_copy_failed", "无法备份本地库", e))?;
        }
    }

    let deployed = match mods_path {
        Some(mods) => find_deployed_dir(mods, &id)?,
        None => None,
    };
    let deploy_backup =
        std::env::temp_dir().join(format!("svmm-deploy-backup-{}", uuid::Uuid::new_v4()));
    if let Some(dir) = &deployed {
        copy_dir_filtered(dir, &deploy_backup)?;
        capture_userdata(userdata_root, &id, dir, &old_files)?;
    }

    let imported = match import_mod_dir(library_root, source, meta) {
        Ok(item) => item,
        Err(err) => {
            restore_tree_backup(&library_backup, &mod_dir(library_root, &id), had_library)?;
            let _ = fs::remove_dir_all(&library_backup);
            let _ = fs::remove_dir_all(&deploy_backup);
            return Err(err);
        }
    };

    if let Some(dir) = &deployed {
        if let Err(err) = (|| -> AppResult<()> {
            remove_dir_if_exists(dir)?;
            if fail_after_deploy_delete {
                return Err(AppError::new("deploy_sync_failed", "替换已部署模组失败"));
            }
            copy_dir_filtered(&mod_dir(library_root, &imported.id), dir)?;
            restore_userdata(userdata_root, &imported.id, dir)?;
            Ok(())
        })() {
            let lib_restore =
                restore_tree_backup(&library_backup, &mod_dir(library_root, &id), had_library);
            let deploy_restore = restore_tree_backup(&deploy_backup, dir, true);
            let _ = fs::remove_dir_all(&library_backup);
            let _ = fs::remove_dir_all(&deploy_backup);
            if lib_restore.is_err() || deploy_restore.is_err() {
                return Err(AppError::new(
                    "deploy_sync_restore_failed",
                    "换版本失败，且无法完整恢复",
                ));
            }
            return Err(err);
        }
    }

    let _ = fs::remove_dir_all(&library_backup);
    let _ = fs::remove_dir_all(&deploy_backup);
    Ok(imported)
}

fn restore_tree_backup(backup: &Path, original: &Path, should_exist: bool) -> AppResult<()> {
    if !should_exist {
        remove_dir_if_exists(original)?;
        return Ok(());
    }
    if !backup.exists() {
        return Err(AppError::new("restore_failed", "找不到备份"));
    }
    remove_dir_if_exists(original)?;
    if let Some(parent) = original.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err("restore_failed", "无法恢复目录", e))?;
    }
    copy_dir_filtered(backup, original)?;
    if backup.join(PACKAGE_FILE).is_file() {
        fs::copy(backup.join(PACKAGE_FILE), original.join(PACKAGE_FILE))
            .map_err(|e| io_err("restore_failed", "无法恢复本地库记录", e))?;
    }
    Ok(())
}

pub fn deploy_one(
    library_root: &Path,
    userdata_root: &Path,
    mods_path: &Path,
    id: &str,
) -> AppResult<PathBuf> {
    let record = load_mod(library_root, id)?;
    if is_smapi_bundled(&record.folder_name, Some(&record.id)) {
        return Err(AppError::new("smapi_bundled", "不能部署 SMAPI 自带模组"));
    }
    let dest = mods_path.join(&record.folder_name);
    remove_dir_if_exists(&dest)?;
    copy_dir_filtered(&mod_dir(library_root, id), &dest)?;
    restore_userdata(userdata_root, id, &dest)?;
    Ok(dest)
}

fn player_mod_dirs(mods_path: &Path) -> AppResult<Vec<(String, PathBuf, bool)>> {
    if !mods_path.is_dir() {
        return Err(AppError::new("mods_dir_missing", "未找到 Mods 目录")
            .with_detail(mods_path.display().to_string()));
    }
    let entries = scan::scan_mods(mods_path)?;
    let mut dirs = Vec::new();
    for entry in entries {
        let mut path = mods_path.to_path_buf();
        for part in entry.folder_path.split(['/', '\\']) {
            if !part.is_empty() && part != "." && part != ".." {
                path.push(part);
            }
        }
        let folder = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if is_smapi_bundled(&folder, Some(&entry.id)) {
            continue;
        }
        dirs.push((entry.id, path, entry.enabled));
    }
    dirs.sort_by(|a, b| b.1.components().count().cmp(&a.1.components().count()));
    Ok(dirs)
}

pub fn profile_is_applied(
    mods_path: &Path,
    library_root: &Path,
    profile: &Profile,
) -> AppResult<bool> {
    if !mods_path.is_dir() {
        return Ok(profile.mod_ids.is_empty());
    }
    let entries = scan::scan_mods(mods_path)?;
    let mut deployed: HashMap<String, String> = HashMap::new();
    for entry in entries {
        let folder = entry
            .folder_path
            .split(['/', '\\'])
            .next_back()
            .unwrap_or("");
        if is_smapi_bundled(folder, Some(&entry.id)) {
            continue;
        }
        deployed.insert(entry.id, entry.version);
    }
    let wanted: HashSet<&str> = profile.mod_ids.iter().map(String::as_str).collect();
    if deployed.len() != wanted.len() || deployed.keys().any(|id| !wanted.contains(id.as_str())) {
        return Ok(false);
    }
    for id in &profile.mod_ids {
        let record = load_mod(library_root, id)?;
        let version = deployed.get(id).map(String::as_str).unwrap_or("");
        if version != record.version {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn apply_profile(
    mods_path: &Path,
    library_root: &Path,
    userdata_root: &Path,
    marker_path: &Path,
    profile: &Profile,
    fail_after_wipe: bool,
) -> AppResult<ApplyReport> {
    for id in &profile.mod_ids {
        load_mod(library_root, id)?;
    }
    let players = player_mod_dirs(mods_path)?;
    let backup = std::env::temp_dir().join(format!("svmm-apply-backup-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&backup)
        .map_err(|e| io_err("apply_backup_failed", "无法备份 Mods 目录", e))?;
    for (_id, path, _enabled) in &players {
        let name = path.file_name().unwrap_or_default();
        copy_dir_filtered(path, &backup.join(name))?;
    }
    let marker = ApplyMarker {
        mods_path: mods_path.display().to_string(),
        backup_path: backup.display().to_string(),
    };
    if let Some(parent) = marker_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(
        marker_path,
        serde_json::to_string(&marker).unwrap_or_else(|_| "{}".to_string()),
    )
    .map_err(|e| io_err("apply_marker_failed", "无法记录应用进度", e))?;

    let library_index: HashMap<String, LibraryMod> =
        list_mods(library_root, LibraryQuery::default())?
            .into_iter()
            .map(|m| (m.id.clone(), m))
            .collect();

    let restore = |err: AppError| -> AppResult<ApplyReport> {
        if let Err(restore_err) = restore_apply_backup(mods_path, &backup) {
            let _ = fs::remove_file(marker_path);
            return Err(
                AppError::new("apply_restore_failed", "应用失败且无法恢复 Mods 目录")
                    .with_detail(format!("{err}; {restore_err}")),
            );
        }
        let _ = fs::remove_dir_all(&backup);
        let _ = fs::remove_file(marker_path);
        Err(err)
    };

    for (id, path, _) in &players {
        if let Some(record) = library_index.get(id) {
            if let Err(err) = capture_userdata(userdata_root, id, path, &record.files) {
                return restore(err);
            }
        }
    }
    for (_id, path, _) in &players {
        if let Err(err) = remove_dir_if_exists(path) {
            return restore(err);
        }
    }
    if fail_after_wipe {
        return restore(AppError::new("apply_failed", "部署方案组失败"));
    }

    let mut deployed = 0usize;
    for id in &profile.mod_ids {
        if let Err(err) = deploy_one(library_root, userdata_root, mods_path, id) {
            return restore(err);
        }
        deployed += 1;
    }
    let _ = fs::remove_dir_all(&backup);
    let _ = fs::remove_file(marker_path);
    Ok(ApplyReport {
        deployed,
        removed: players.len(),
        errors: Vec::new(),
    })
}

fn restore_apply_backup(mods_path: &Path, backup: &Path) -> AppResult<()> {
    let current = player_mod_dirs(mods_path).unwrap_or_default();
    for (_id, path, _) in current {
        remove_dir_if_exists(&path)?;
    }
    if !backup.is_dir() {
        return Ok(());
    }
    let entries =
        fs::read_dir(backup).map_err(|e| io_err("apply_restore_failed", "无法读取备份", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| io_err("apply_restore_failed", "无法读取备份", e))?;
        let from = entry.path();
        let to = mods_path.join(entry.file_name());
        if from.is_dir() {
            copy_dir_filtered(&from, &to)?;
        }
    }
    Ok(())
}

pub fn resume_incomplete_apply(marker_path: &Path) -> AppResult<bool> {
    if !marker_path.is_file() {
        return Ok(false);
    }
    let text = fs::read_to_string(marker_path)
        .map_err(|e| io_err("apply_marker_failed", "无法读取未完成的应用", e))?;
    let marker: ApplyMarker = serde_json::from_str(&text).map_err(|e| {
        AppError::new("apply_marker_failed", "未完成的应用记录无效").with_detail(e.to_string())
    })?;
    let mods_path = PathBuf::from(&marker.mods_path);
    let backup = PathBuf::from(&marker.backup_path);
    restore_apply_backup(&mods_path, &backup)?;
    let _ = fs::remove_dir_all(&backup);
    let _ = fs::remove_file(marker_path);
    Ok(true)
}

pub fn seed_userdata_config(userdata_root: &Path, id: &str, mod_dir_path: &Path) -> AppResult<()> {
    let config = mod_dir_path.join("config.json");
    if !config.is_file() {
        return Ok(());
    }
    let dest = userdata_root.join(safe_component(id)).join("config.json");
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| io_err("userdata_write_failed", "无法保存玩家数据", e))?;
    }
    fs::copy(&config, &dest).map_err(|e| io_err("userdata_write_failed", "无法保存玩家数据", e))?;
    Ok(())
}

pub fn migrate_installed_mods(
    mods_path: &Path,
    library_root: &Path,
    userdata_root: &Path,
    profiles_dir: &Path,
    now_iso: &str,
) -> AppResult<()> {
    if !mods_path.is_dir() {
        crate::storage::profiles_store::ensure_default_profile(profiles_dir, Vec::new(), now_iso)?;
        return Ok(());
    }
    let existing_ids: HashSet<String> = list_mods(library_root, LibraryQuery::default())?
        .into_iter()
        .map(|m| m.id)
        .collect();
    let dismissed = read_dismissed(library_root)?;
    let players = player_mod_dirs(mods_path).unwrap_or_default();
    let mut imported_enabled = Vec::new();
    for (id, path, enabled) in &players {
        if existing_ids.contains(id) || is_dismissed(&dismissed, id) {
            continue;
        }
        if is_smapi_bundled(
            path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
            Some(id),
        ) {
            continue;
        }
        let Ok(imported) = import_mod_dir(library_root, path, ImportMeta::default()) else {
            continue;
        };
        let _ = seed_userdata_config(userdata_root, &imported.id, path);
        if *enabled {
            imported_enabled.push(imported.id);
        }
    }
    crate::storage::profiles_store::ensure_default_profile(
        profiles_dir,
        imported_enabled.clone(),
        now_iso,
    )?;
    if let Ok(mut profile) =
        crate::storage::profiles_store::load_profile_by_id_from(profiles_dir, "default")
    {
        let mut changed = false;
        for id in imported_enabled {
            if !profile.mod_ids.iter().any(|item| item == &id) {
                profile.mod_ids.push(id);
                changed = true;
            }
        }
        if changed {
            profile.updated_at = now_iso.to_string();
            crate::storage::profiles_store::save_profile_to(profiles_dir, &profile)?;
        }
    }
    let known: HashSet<String> = list_mods(library_root, LibraryQuery::default())?
        .into_iter()
        .map(|m| m.id)
        .collect();
    if profiles_dir.exists() {
        for mut profile in crate::storage::profiles_store::list_profiles_from(profiles_dir)? {
            profile.mod_ids.retain(|id| known.contains(id));
            crate::storage::profiles_store::save_profile_to(profiles_dir, &profile)?;
        }
    }
    Ok(())
}

/// Manifest-backed entries used to ask SMAPI which library packages can update.
pub fn entries_for_updates(library_root: &Path) -> AppResult<Vec<scan::ModEntry>> {
    let mods = list_mods(library_root, LibraryQuery::default())?;
    let mut entries = Vec::new();
    for item in mods {
        let manifest_path = mod_dir(library_root, &item.id).join("manifest.json");
        let Ok(bytes) = fs::read(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = parse_manifest(&bytes) else {
            continue;
        };
        entries.push(scan::ModEntry {
            id: item.id,
            name: item.name,
            author: item.author,
            version: item.version,
            description: item.description,
            folder_path: item.folder_name,
            enabled: true,
            minimum_api_version: manifest.minimum_api_version,
            update_keys: manifest.update_keys,
            dependencies: manifest
                .dependencies
                .iter()
                .map(scan::ModDependency::from)
                .collect(),
            status: "ok".into(),
        });
    }
    Ok(entries)
}

pub fn profile_state(
    mods_path: Option<&Path>,
    library_root: &Path,
    profile: &Profile,
) -> AppResult<ProfileState> {
    let mut mods = Vec::new();
    for id in &profile.mod_ids {
        if let Ok(item) = load_mod(library_root, id) {
            mods.push(item);
        }
    }
    let applied = match mods_path {
        Some(path) if path.is_dir() => profile_is_applied(path, library_root, profile)?,
        _ => false,
    };
    Ok(ProfileState {
        profile: profile.clone(),
        mods,
        applied,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
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

    fn write_mod(dir: &Path, version: &str, extra: Option<(&str, &str)>) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("manifest.json"),
            format!(
                r#"{{
  "Name": "Demo",
  "Author": "Ada",
  "Version": "{version}",
  "Description": "Hi",
  "UniqueID": "Ada.Demo",
  "UpdateKeys": ["Nexus:42"]
}}"#
            ),
        )
        .unwrap();
        fs::write(dir.join("mod.dll"), b"v-bytes").unwrap();
        if let Some((name, body)) = extra {
            fs::write(dir.join(name), body).unwrap();
        }
    }

    #[test]
    fn import_writes_manifest_and_replace_drops_old_files() {
        let root = temp_dir("svmm-lib");
        let library = root.join("library");
        let src = root.join("Demo");
        write_mod(&src, "1.0.0", Some(("old.txt", "gone")));
        let first = import_mod_dir(&library, &src, ImportMeta::default()).unwrap();
        assert!(library
            .join(safe_component(&first.id))
            .join(PACKAGE_FILE)
            .is_file());
        assert!(first.files.iter().any(|f| f.path == "old.txt"));
        assert_eq!(first.nexus_mod_id, Some(42));

        fs::remove_file(src.join("old.txt")).unwrap();
        fs::write(src.join("manifest.json"), include_newer()).unwrap();
        let second = import_mod_dir(&library, &src, ImportMeta::default()).unwrap();
        assert_eq!(second.version, "2.0.0");
        assert!(!second.files.iter().any(|f| f.path == "old.txt"));
        assert!(!library
            .join(safe_component(&second.id))
            .join("old.txt")
            .exists());
        let _ = fs::remove_dir_all(&root);
    }

    fn include_newer() -> String {
        r#"{
  "Name": "Demo",
  "Author": "Ada",
  "Version": "2.0.0",
  "Description": "Hi",
  "UniqueID": "Ada.Demo"
}"#
        .to_string()
    }

    #[test]
    fn skips_smapi_bundled_mods() {
        let root = temp_dir("svmm-smapi");
        let library = root.join("library");
        let src = root.join("ConsoleCommands");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("manifest.json"),
            r#"{
  "Name": "Console Commands",
  "Author": "SMAPI",
  "Version": "4.0.0",
  "Description": "bundled",
  "UniqueID": "SMAPI.ConsoleCommands"
}"#,
        )
        .unwrap();
        let err = import_mod_dir(&library, &src, ImportMeta::default()).unwrap_err();
        assert_eq!(err.code, "smapi_bundled");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn zip_import_does_not_touch_mods_dir() {
        let root = temp_dir("svmm-zip-lib");
        let library = root.join("library");
        let mods = root.join("Mods");
        fs::create_dir_all(&mods).unwrap();
        fs::write(mods.join("keep.txt"), b"stay").unwrap();
        let zip_path = root.join("mod.zip");
        write_zip(
            &zip_path,
            &[
                ("Ada.Demo/", b""),
                (
                    "Ada.Demo/manifest.json",
                    br#"{
  "Name": "Demo",
  "Author": "Ada",
  "Version": "1.0.0",
  "Description": "Hi",
  "UniqueID": "Ada.Demo"
}"#,
                ),
            ],
        );
        let imported = import_zip(&zip_path, &library, ImportMeta::default()).unwrap();
        assert_eq!(imported.id, "Ada.Demo");
        assert!(mods.join("keep.txt").is_file());
        assert!(!mods.join("Ada.Demo").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn zip_imports_every_sibling_mod() {
        let root = temp_dir("svmm-zip-sve");
        let library = root.join("library");
        let zip_path = root.join("sve.zip");
        write_zip(
            &zip_path,
            &[
                (
                    "Stardew Valley Expanded/[CP] Stardew Valley Expanded/manifest.json",
                    br#"{
  "Name": "Stardew Valley Expanded",
  "Author": "FlashShifter",
  "Version": "1.15.0",
  "Description": "cp",
  "UniqueID": "FlashShifter.StardewValleyExpanded"
}"#,
                ),
                (
                    "Stardew Valley Expanded/[FTM] Stardew Valley Expanded/manifest.json",
                    br#"{
  "Name": "Stardew Valley Expanded Farm Type",
  "Author": "FlashShifter",
  "Version": "1.15.0",
  "Description": "ftm",
  "UniqueID": "FlashShifter.StardewValleyExpanded.FTM"
}"#,
                ),
            ],
        );
        import_zip(&zip_path, &library, ImportMeta::default()).unwrap();
        let mods = list_mods(&library, LibraryQuery::default()).unwrap();
        let mut ids: Vec<_> = mods.iter().map(|item| item.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(
            ids,
            vec![
                "FlashShifter.StardewValleyExpanded",
                "FlashShifter.StardewValleyExpanded.FTM",
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn zip_keeps_nested_content_pack_inside_parent() {
        let root = temp_dir("svmm-zip-nested");
        let library = root.join("library");
        let zip_path = root.join("mod.zip");
        write_zip(
            &zip_path,
            &[
                (
                    "MyMod/manifest.json",
                    br#"{
  "Name": "Parent",
  "Author": "Ada",
  "Version": "1.0.0",
  "Description": "parent",
  "UniqueID": "Ada.Parent"
}"#,
                ),
                (
                    "MyMod/[CP] Extra/manifest.json",
                    br#"{
  "Name": "Extra",
  "Author": "Ada",
  "Version": "1.0.0",
  "Description": "child",
  "UniqueID": "Ada.Parent.CP"
}"#,
                ),
            ],
        );
        import_zip(&zip_path, &library, ImportMeta::default()).unwrap();
        let mods = list_mods(&library, LibraryQuery::default()).unwrap();
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].id, "Ada.Parent");
        assert!(library
            .join("Ada.Parent")
            .join("[CP] Extra")
            .join("manifest.json")
            .is_file());
        let _ = fs::remove_dir_all(&root);
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;
        let file = fs::File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, data) in entries {
            if name.ends_with('/') {
                zip.add_directory(*name, opts).unwrap();
            } else {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(data).unwrap();
            }
        }
        zip.finish().unwrap();
    }

    #[test]
    fn userdata_survives_reapply() {
        let root = temp_dir("svmm-user");
        let library = root.join("library");
        let userdata = root.join("userdata");
        let mods = root.join("Mods");
        fs::create_dir_all(&mods).unwrap();
        let src = root.join("Demo");
        write_mod(&src, "1.0.0", Some(("config.json", "{\"ok\":false}")));
        import_mod_dir(&library, &src, ImportMeta::default()).unwrap();
        let profiles = root.join("profiles");
        let profile = Profile {
            id: "default".into(),
            name: "默认方案".into(),
            created_at: "t".into(),
            updated_at: "t".into(),
            mod_ids: vec!["Ada.Demo".into()],
        };
        apply_profile(
            &mods,
            &library,
            &userdata,
            &root.join("marker.json"),
            &profile,
            false,
        )
        .unwrap();
        fs::write(mods.join("Demo").join("config.json"), b"{\"ok\":true}").unwrap();

        let empty = Profile {
            id: "other".into(),
            name: "空".into(),
            created_at: "t".into(),
            updated_at: "t".into(),
            mod_ids: vec![],
        };
        apply_profile(
            &mods,
            &library,
            &userdata,
            &root.join("marker.json"),
            &empty,
            false,
        )
        .unwrap();
        assert!(!mods.join("Demo").exists());
        apply_profile(
            &mods,
            &library,
            &userdata,
            &root.join("marker.json"),
            &profile,
            false,
        )
        .unwrap();
        let config = fs::read_to_string(mods.join("Demo").join("config.json")).unwrap();
        assert_eq!(config, "{\"ok\":true}");
        let _ = fs::remove_dir_all(&profiles);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn apply_failure_restores_mods_and_keeps_smapi() {
        let root = temp_dir("svmm-apply-fail");
        let library = root.join("library");
        let userdata = root.join("userdata");
        let mods = root.join("Mods");
        let bundled = mods.join("ConsoleCommands");
        fs::create_dir_all(&bundled).unwrap();
        fs::write(bundled.join("manifest.json"), smapi_manifest()).unwrap();
        fs::write(bundled.join("keep.dll"), b"smapi").unwrap();
        let player = mods.join("OldMod");
        write_mod(&player, "1.0.0", None);
        fs::write(
            player.join("manifest.json"),
            r#"{
  "Name": "Old",
  "Author": "Ada",
  "Version": "1.0.0",
  "Description": "Hi",
  "UniqueID": "Ada.Old"
}"#,
        )
        .unwrap();
        let src = root.join("Demo");
        write_mod(&src, "1.0.0", None);
        import_mod_dir(&library, &src, ImportMeta::default()).unwrap();
        let profile = Profile {
            id: "default".into(),
            name: "默认方案".into(),
            created_at: "t".into(),
            updated_at: "t".into(),
            mod_ids: vec!["Ada.Demo".into()],
        };
        let err = apply_profile(
            &mods,
            &library,
            &userdata,
            &root.join("marker.json"),
            &profile,
            true,
        )
        .unwrap_err();
        assert_eq!(err.code, "apply_failed");
        assert!(mods.join("OldMod").join("manifest.json").is_file());
        assert!(mods.join("ConsoleCommands").join("keep.dll").is_file());
        assert!(!root.join("marker.json").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    fn smapi_manifest() -> &'static str {
        r#"{
  "Name": "Console Commands",
  "Author": "SMAPI",
  "Version": "4.0.0",
  "Description": "bundled",
  "UniqueID": "SMAPI.ConsoleCommands"
}"#
    }

    #[test]
    fn version_swap_failure_restores_library_and_deployed_copy() {
        let root = temp_dir("svmm-swap");
        let library = root.join("library");
        let userdata = root.join("userdata");
        let mods = root.join("Mods");
        fs::create_dir_all(&mods).unwrap();
        let src = root.join("Demo");
        write_mod(&src, "1.0.0", Some(("note.txt", "old")));
        import_mod_dir(&library, &src, ImportMeta::default()).unwrap();
        let profile = Profile {
            id: "default".into(),
            name: "默认方案".into(),
            created_at: "t".into(),
            updated_at: "t".into(),
            mod_ids: vec!["Ada.Demo".into()],
        };
        apply_profile(
            &mods,
            &library,
            &userdata,
            &root.join("marker.json"),
            &profile,
            false,
        )
        .unwrap();

        fs::write(src.join("manifest.json"), include_newer()).unwrap();
        fs::write(src.join("note.txt"), b"new").unwrap();
        let err = replace_package(
            &library,
            &userdata,
            Some(&mods),
            &src,
            ImportMeta::default(),
            true,
        )
        .unwrap_err();
        assert_eq!(err.code, "deploy_sync_failed");
        let current = load_mod(&library, "Ada.Demo").unwrap();
        assert_eq!(current.version, "1.0.0");
        assert_eq!(
            fs::read_to_string(mods.join("Demo").join("note.txt")).unwrap(),
            "old"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn replace_does_not_keep_previous_package() {
        let root = temp_dir("svmm-down");
        let library = root.join("library");
        let src = root.join("Demo");
        write_mod(&src, "2.0.0", Some(("only-new.txt", "x")));
        import_mod_dir(&library, &src, ImportMeta::default()).unwrap();
        fs::remove_file(src.join("only-new.txt")).unwrap();
        fs::write(
            src.join("manifest.json"),
            r#"{
  "Name": "Demo",
  "Author": "Ada",
  "Version": "1.0.0",
  "Description": "Hi",
  "UniqueID": "Ada.Demo"
}"#,
        )
        .unwrap();
        fs::write(src.join("only-old.txt"), b"y").unwrap();
        let replaced = replace_package(
            &library,
            &root.join("userdata"),
            None,
            &src,
            ImportMeta {
                nexus_file_id: Some(7),
                ..ImportMeta::default()
            },
            false,
        )
        .unwrap();
        assert_eq!(replaced.version, "1.0.0");
        assert!(!library
            .join(safe_component("Ada.Demo"))
            .join("only-new.txt")
            .exists());
        assert!(replaced.files.iter().any(|f| f.path == "only-old.txt"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn filter_by_category_id_and_keyword() {
        let root = temp_dir("svmm-filter");
        let library = root.join("library");
        let src = root.join("Demo");
        write_mod(&src, "1.0.0", None);
        import_mod_dir(
            &library,
            &src,
            ImportMeta {
                category: Some("Gameplay".into()),
                ..ImportMeta::default()
            },
        )
        .unwrap();
        let by_cat = list_mods(
            &library,
            LibraryQuery {
                category: Some("Gameplay"),
                ..LibraryQuery::default()
            },
        )
        .unwrap();
        assert_eq!(by_cat.len(), 1);
        let by_id = list_mods(
            &library,
            LibraryQuery {
                id: Some("Ada.Demo"),
                ..LibraryQuery::default()
            },
        )
        .unwrap();
        assert_eq!(by_id.len(), 1);
        let by_kw = list_mods(
            &library,
            LibraryQuery {
                keyword: Some("ada"),
                ..LibraryQuery::default()
            },
        )
        .unwrap();
        assert_eq!(by_kw.len(), 1);
        let miss = list_mods(
            &library,
            LibraryQuery {
                keyword: Some("nope"),
                ..LibraryQuery::default()
            },
        )
        .unwrap();
        assert!(miss.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resume_restores_mods_after_interrupted_apply() {
        let root = temp_dir("svmm-resume");
        let mods = root.join("Mods");
        let player = mods.join("OldMod");
        write_mod(&player, "1.0.0", Some(("keep.txt", "yes")));
        let backup = root.join("backup").join("OldMod");
        fs::create_dir_all(&backup).unwrap();
        fs::copy(player.join("manifest.json"), backup.join("manifest.json")).unwrap();
        fs::copy(player.join("keep.txt"), backup.join("keep.txt")).unwrap();
        fs::remove_dir_all(&player).unwrap();
        let marker = root.join("marker.json");
        fs::write(
            &marker,
            serde_json::json!({
                "modsPath": mods.display().to_string(),
                "backupPath": root.join("backup").display().to_string(),
            })
            .to_string(),
        )
        .unwrap();
        assert!(resume_incomplete_apply(&marker).unwrap());
        assert_eq!(
            fs::read_to_string(mods.join("OldMod").join("keep.txt")).unwrap(),
            "yes"
        );
        assert!(!marker.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn status_ignores_mods_folder_only_copies() {
        let mods = vec![LibraryMod {
            id: "Ada.Demo".into(),
            name: "Demo".into(),
            author: "Ada".into(),
            version: "1.0.0".into(),
            description: String::new(),
            id_from_manifest: true,
            category: None,
            nexus_mod_id: Some(42),
            nexus_file_id: None,
            folder_name: "Demo".into(),
            files: vec![],
        }];
        assert_eq!(nexus_library_status(&mods, 99, "1.0.0"), "missing");
        assert_eq!(nexus_library_status(&mods, 42, "1.0.0"), "owned");
        assert_eq!(nexus_library_status(&mods, 42, "1.2.0"), "update");
    }

    #[test]
    fn migrate_imports_game_mods_when_library_already_has_entries() {
        let root = temp_dir("svmm-adopt");
        let library = root.join("library");
        let userdata = root.join("userdata");
        let profiles = root.join("profiles");
        let already = root.join("Already");
        write_mod(&already, "1.0.0", None);
        import_mod_dir(&library, &already, ImportMeta::default()).unwrap();
        crate::storage::profiles_store::ensure_default_profile(
            &profiles,
            vec!["Ada.Demo".into()],
            "t0",
        )
        .unwrap();

        let mods = root.join("Mods");
        let installed = mods.join("Extra");
        fs::create_dir_all(&installed).unwrap();
        fs::write(
            installed.join("manifest.json"),
            r#"{
  "Name": "Extra",
  "Author": "Bea",
  "Version": "3.0.0",
  "Description": "x",
  "UniqueID": "Bea.Extra"
}"#,
        )
        .unwrap();
        fs::write(installed.join("mod.dll"), b"x").unwrap();

        migrate_installed_mods(&mods, &library, &userdata, &profiles, "t1").unwrap();
        let ids: Vec<String> = list_mods(&library, LibraryQuery::default())
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert!(ids.iter().any(|id| id == "Ada.Demo"));
        assert!(ids.iter().any(|id| id == "Bea.Extra"));
        let profile =
            crate::storage::profiles_store::load_profile_by_id_from(&profiles, "default").unwrap();
        assert!(profile.mod_ids.iter().any(|id| id == "Bea.Extra"));
        assert!(installed.is_dir());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn delete_drops_profile_membership_and_does_not_come_back() {
        let root = temp_dir("svmm-del");
        let library = root.join("library");
        let userdata = root.join("userdata");
        let profiles = root.join("profiles");
        let src = root.join("Demo");
        write_mod(&src, "1.0.0", None);
        import_mod_dir(&library, &src, ImportMeta::default()).unwrap();

        crate::storage::profiles_store::ensure_default_profile(
            &profiles,
            vec!["ada.demo".into()],
            "t0",
        )
        .unwrap();
        crate::storage::profiles_store::save_profile_to(
            &profiles,
            &Profile {
                id: "other".into(),
                name: "其他".into(),
                created_at: "t0".into(),
                updated_at: "t0".into(),
                mod_ids: vec!["Ada.Demo".into(), "Keep.Me".into()],
            },
        )
        .unwrap();

        let mods = root.join("Mods");
        let deployed = mods.join("Demo");
        fs::create_dir_all(&deployed).unwrap();
        fs::copy(src.join("manifest.json"), deployed.join("manifest.json")).unwrap();
        fs::write(deployed.join("mod.dll"), b"v-bytes").unwrap();

        delete_mod(&library, &profiles, "Ada.Demo").unwrap();
        assert!(list_mods(&library, LibraryQuery::default())
            .unwrap()
            .is_empty());

        let default_profile =
            crate::storage::profiles_store::load_profile_by_id_from(&profiles, "default").unwrap();
        assert!(!default_profile
            .mod_ids
            .iter()
            .any(|id| id.eq_ignore_ascii_case("Ada.Demo")));
        let other =
            crate::storage::profiles_store::load_profile_by_id_from(&profiles, "other").unwrap();
        assert_eq!(other.mod_ids, vec!["Keep.Me".to_string()]);

        migrate_installed_mods(&mods, &library, &userdata, &profiles, "t1").unwrap();
        assert!(list_mods(&library, LibraryQuery::default())
            .unwrap()
            .is_empty());
        let default_again =
            crate::storage::profiles_store::load_profile_by_id_from(&profiles, "default").unwrap();
        assert!(!default_again
            .mod_ids
            .iter()
            .any(|id| id.eq_ignore_ascii_case("Ada.Demo")));
        assert!(deployed.is_dir());
        let _ = fs::remove_dir_all(&root);
    }
}
