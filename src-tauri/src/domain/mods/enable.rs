use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::mods::scan::{entry_from_mod_dir, ModEntry};
use crate::error::{AppError, AppResult};

/// Resolve `relative` under `mods_path`, accepting `/` or `\` separators.
fn resolve_mod_dir(mods_path: &Path, relative: &str) -> PathBuf {
    let mut path = mods_path.to_path_buf();
    for part in relative.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            // Refuse path escape; keep under mods_path.
            continue;
        }
        path.push(part);
    }
    path
}

fn leaf_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Enable/disable by renaming the leaf folder: disable adds a leading `.`, enable removes it.
/// Returns the updated [`ModEntry`] after rename.
pub fn set_mod_enabled(mods_path: &Path, folder_path: &str, enabled: bool) -> AppResult<ModEntry> {
    let current = resolve_mod_dir(mods_path, folder_path);
    if !current.is_dir() {
        return Err(AppError::new("mod_not_found", "未找到指定 mod 目录")
            .with_detail(current.display().to_string()));
    }

    let name = leaf_name(&current);
    if name.is_empty() {
        return Err(AppError::new("mod_path_invalid", "无效的 mod 路径")
            .with_detail(current.display().to_string()));
    }

    let new_name = if enabled {
        name.strip_prefix('.').unwrap_or(&name).to_string()
    } else if name.starts_with('.') {
        name
    } else {
        format!(".{name}")
    };

    let parent = current.parent().ok_or_else(|| {
        AppError::new("mod_path_invalid", "无效的 mod 路径")
            .with_detail(current.display().to_string())
    })?;
    let new_path = parent.join(&new_name);

    if current != new_path {
        if new_path.exists() {
            return Err(
                AppError::new("mod_rename_conflict", "目标目录已存在，无法切换启用状态")
                    .with_detail(new_path.display().to_string()),
            );
        }
        fs::rename(&current, &new_path).map_err(|e| {
            AppError::new("mod_rename_failed", "无法重命名 mod 目录以切换启用状态")
                .with_detail(e.to_string())
        })?;
    }

    entry_from_mod_dir(mods_path, &new_path)
}
