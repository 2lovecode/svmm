use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};

use zip::ZipArchive;

use crate::domain::mods::scan::{entry_from_mod_dir, ModEntry};
use crate::error::{AppError, AppResult};

fn unsafe_zip(detail: impl Into<String>) -> AppError {
    AppError::new("unsafe_zip", "压缩包包含不安全路径").with_detail(detail)
}

/// Reject `..`, absolute paths, and any entry that would escape `dest`.
pub fn validate_zip_entry_path(name: &str, dest: &Path) -> AppResult<PathBuf> {
    if name.is_empty() {
        return Err(unsafe_zip("empty entry name"));
    }
    // Zip uses `/` separators; reject Windows drive / UNC style names early.
    if name.starts_with('/') || name.starts_with('\\') {
        return Err(unsafe_zip(name));
    }
    if name.contains(':') {
        return Err(unsafe_zip(name));
    }

    let entry = Path::new(name);
    if entry.is_absolute() {
        return Err(unsafe_zip(name));
    }

    for component in entry.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(unsafe_zip(name));
            }
        }
    }

    let dest_abs = normalize_for_compare(dest);
    let out = dest.join(entry);
    let out_abs = normalize_for_compare(&out);
    if !out_abs.starts_with(&dest_abs) {
        return Err(unsafe_zip(name));
    }
    Ok(out)
}

fn normalize_for_compare(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Safely extract `zip_path` into `dest_mods` and return every installed mod folder.
///
/// A package may wrap several mods (Stardew Valley Expanded ships a content pack
/// and a farm type map). Nested manifests inside a mod folder stay with that mod.
pub fn safe_extract_zip(zip_path: &Path, dest_mods: &Path) -> AppResult<Vec<PathBuf>> {
    if !dest_mods.is_dir() {
        return Err(AppError::new("mods_dir_missing", "未找到 Mods 目录")
            .with_detail(dest_mods.display().to_string()));
    }
    if !zip_path.is_file() {
        return Err(AppError::new("zip_missing", "未找到压缩包")
            .with_detail(zip_path.display().to_string()));
    }

    let file = File::open(zip_path).map_err(|e| {
        AppError::new("zip_open_failed", "无法打开压缩包").with_detail(e.to_string())
    })?;
    let mut archive = ZipArchive::new(file).map_err(|e| {
        AppError::new("zip_invalid", "无效的 zip 压缩包").with_detail(e.to_string())
    })?;

    let staging = dest_mods.join(format!(".svmm-extract-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&staging).map_err(|e| {
        AppError::new("zip_extract_failed", "无法创建解压目录").with_detail(e.to_string())
    })?;

    let extract_result = (|| -> AppResult<Vec<PathBuf>> {
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| {
                AppError::new("zip_read_failed", "无法读取压缩包条目").with_detail(e.to_string())
            })?;
            // Prefer enclosed_name (zip crate strips some unsafe forms); fall back to raw name.
            let name = match entry.enclosed_name() {
                Some(p) => p.to_string_lossy().replace('\\', "/"),
                None => {
                    // enclosed_name rejected the path — still validate raw name for our error code.
                    let raw = entry.name().replace('\\', "/");
                    return Err(unsafe_zip(raw));
                }
            };
            let is_dir = entry.is_dir() || name.ends_with('/');
            if is_dir {
                let dir_name = name.trim_end_matches('/');
                if !dir_name.is_empty() {
                    let dir = validate_zip_entry_path(dir_name, &staging)?;
                    fs::create_dir_all(&dir).map_err(|e| {
                        AppError::new("zip_extract_failed", "无法创建目录")
                            .with_detail(e.to_string())
                    })?;
                }
                continue;
            }

            let out_path = validate_zip_entry_path(&name, &staging)?;
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    AppError::new("zip_extract_failed", "无法创建目录").with_detail(e.to_string())
                })?;
            }
            let mut outfile = File::create(&out_path).map_err(|e| {
                AppError::new("zip_extract_failed", "无法写入解压文件").with_detail(e.to_string())
            })?;
            io::copy(&mut entry, &mut outfile).map_err(|e| {
                AppError::new("zip_extract_failed", "解压失败").with_detail(e.to_string())
            })?;
        }

        let roots = collect_mod_roots(&staging);
        if roots.is_empty() {
            return Err(AppError::new("zip_no_mod", "压缩包中未找到有效的模组目录"));
        }
        let mut installed = Vec::with_capacity(roots.len());
        for mod_root in roots {
            let folder_name = unique_mod_folder_name(dest_mods, &mod_root)?;
            let final_path = dest_mods.join(&folder_name);
            fs::rename(&mod_root, &final_path).map_err(|e| {
                AppError::new("zip_install_failed", "无法安装模组目录").with_detail(e.to_string())
            })?;
            installed.push(final_path);
        }
        Ok(installed)
    })();

    let _ = fs::remove_dir_all(&staging);
    extract_result
}

/// Mod folders that SMAPI would load: a directory with `manifest.json`, not nested
/// inside another such directory. `__MACOSX` is ignored.
fn collect_mod_roots(dir: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    collect_mod_roots_into(dir, &mut roots);
    roots.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    roots
}

fn collect_mod_roots_into(dir: &Path, roots: &mut Vec<PathBuf>) {
    if dir.join("manifest.json").is_file() {
        roots.push(dir.to_path_buf());
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        if name.eq_ignore_ascii_case("__MACOSX") {
            continue;
        }
        collect_mod_roots_into(&path, roots);
    }
}

fn unique_mod_folder_name(dest_mods: &Path, mod_root: &Path) -> AppResult<String> {
    let base = if mod_root == dest_mods
        || mod_root
            .file_name()
            .map(|n| n.to_string_lossy().starts_with(".svmm-extract-"))
            .unwrap_or(false)
    {
        // Flat zip (manifest at staging root): derive name from UniqueID or fallback.
        if let Ok(bytes) = fs::read(mod_root.join("manifest.json")) {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                if let Some(id) = v.get("UniqueID").and_then(|x| x.as_str()) {
                    id.replace('.', "_")
                } else if let Some(name) = v.get("Name").and_then(|x| x.as_str()) {
                    sanitize_folder_name(name)
                } else {
                    "InstalledMod".to_string()
                }
            } else {
                "InstalledMod".to_string()
            }
        } else {
            "InstalledMod".to_string()
        }
    } else {
        mod_root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "InstalledMod".to_string())
    };

    let base = sanitize_folder_name(&base);
    let mut candidate = base.clone();
    let mut n = 2u32;
    while dest_mods.join(&candidate).exists() {
        candidate = format!("{base}_{n}");
        n += 1;
    }
    Ok(candidate)
}

fn sanitize_folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_start_matches('.');
    if trimmed.is_empty() {
        "InstalledMod".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn install_mod_zip(zip_path: &Path, mods_path: &Path) -> AppResult<ModEntry> {
    let installed = safe_extract_zip(zip_path, mods_path)?;
    entry_from_mod_dir(mods_path, &installed[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

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

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
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
    fn rejects_path_traversal() {
        let root = temp_dir("svmm-zip-slip");
        let mods = root.join("Mods");
        fs::create_dir_all(&mods).unwrap();
        let zip_path = root.join("evil.zip");
        write_zip(&zip_path, &[("../evil.dll", b"malware")]);

        let err = safe_extract_zip(&zip_path, &mods).unwrap_err();
        assert_eq!(err.code, "unsafe_zip");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_absolute_entry() {
        let dest = PathBuf::from("C:\\games\\Mods");
        let err = validate_zip_entry_path("/tmp/evil.dll", &dest).unwrap_err();
        assert_eq!(err.code, "unsafe_zip");
        let err2 = validate_zip_entry_path("C:/Windows/evil.dll", &dest).unwrap_err();
        assert_eq!(err2.code, "unsafe_zip");
    }

    #[test]
    fn installs_zip_with_manifest_happy_path() {
        let root = temp_dir("svmm-zip-ok");
        let mods = root.join("Mods");
        fs::create_dir_all(&mods).unwrap();
        let zip_path = root.join("mod.zip");
        let manifest = br#"{
  "Name": "Zip Mod",
  "Author": "Ada",
  "Version": "1.2.3",
  "Description": "from zip",
  "UniqueID": "Ada.ZipMod"
}"#;
        write_zip(
            &zip_path,
            &[
                ("Ada.ZipMod/", b""),
                ("Ada.ZipMod/manifest.json", manifest),
                ("Ada.ZipMod/readme.txt", b"hi"),
            ],
        );

        let entry = install_mod_zip(&zip_path, &mods).unwrap();
        assert_eq!(entry.id, "Ada.ZipMod");
        assert_eq!(entry.name, "Zip Mod");
        assert_eq!(entry.version, "1.2.3");
        assert!(mods.join("Ada.ZipMod").join("manifest.json").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extracts_wrapper_zip_into_every_mod() {
        let root = temp_dir("svmm-zip-wrap");
        let mods = root.join("Mods");
        fs::create_dir_all(&mods).unwrap();
        let zip_path = root.join("sve.zip");
        let cp = br#"{
  "Name": "Stardew Valley Expanded",
  "Author": "FlashShifter",
  "Version": "1.15.0",
  "Description": "cp",
  "UniqueID": "FlashShifter.StardewValleyExpanded"
}"#;
        let ftm = br#"{
  "Name": "Stardew Valley Expanded Farm Type",
  "Author": "FlashShifter",
  "Version": "1.15.0",
  "Description": "ftm",
  "UniqueID": "FlashShifter.StardewValleyExpanded.FTM"
}"#;
        write_zip(
            &zip_path,
            &[
                (
                    "Stardew Valley Expanded/[CP] Stardew Valley Expanded/manifest.json",
                    cp,
                ),
                (
                    "Stardew Valley Expanded/[FTM] Stardew Valley Expanded/manifest.json",
                    ftm,
                ),
                ("Stardew Valley Expanded/readme.txt", b"install both folders"),
            ],
        );

        let installed = safe_extract_zip(&zip_path, &mods).unwrap();
        assert_eq!(installed.len(), 2);
        assert!(mods
            .join("[CP] Stardew Valley Expanded")
            .join("manifest.json")
            .is_file());
        assert!(mods
            .join("[FTM] Stardew Valley Expanded")
            .join("manifest.json")
            .is_file());
        assert!(!mods.join("Stardew Valley Expanded").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_rejects_parent_dir_component() {
        let dest = PathBuf::from("C:\\games\\Mods");
        let err = validate_zip_entry_path("foo/../../evil.dll", &dest).unwrap_err();
        assert_eq!(err.code, "unsafe_zip");
    }
}
