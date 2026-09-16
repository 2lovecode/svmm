use crate::domain::game;
use crate::domain::mods::scan;
use crate::domain::profiles::{self, ApplyReport, Profile};
use crate::error::{AppError, AppResult};
use crate::storage::profiles_store;
use crate::storage::settings;

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn enabled_ids_from_scan(mods_path: &std::path::Path) -> AppResult<Vec<String>> {
    let entries = scan::scan_mods(mods_path)?;
    Ok(entries
        .into_iter()
        .filter(|e| e.enabled)
        .map(|e| e.id)
        .collect())
}

fn try_resolve_mods_path() -> AppResult<std::path::PathBuf> {
    let settings = settings::load_settings()?;
    let paths = game::resolve_paths(&settings)?;
    Ok(paths.mods_path)
}

/// List profiles; on first run (empty store) create `default` from currently enabled mods.
#[tauri::command]
pub fn list_profiles() -> AppResult<Vec<Profile>> {
    let dir = profiles_store::profiles_dir();
    let existing = profiles_store::list_profiles_from(&dir)?;
    if !existing.is_empty() {
        return Ok(existing);
    }
    let enabled = match try_resolve_mods_path() {
        Ok(mods) => enabled_ids_from_scan(&mods).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    profiles_store::ensure_default_profile(&dir, enabled, &now_iso())
}

#[tauri::command]
pub fn create_profile(name: String, enabled_mod_ids: Option<Vec<String>>) -> AppResult<Profile> {
    let ids = match enabled_mod_ids {
        Some(ids) => ids,
        None => {
            let mods = try_resolve_mods_path()?;
            enabled_ids_from_scan(&mods)?
        }
    };
    let now = now_iso();
    let profile = Profile {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        created_at: now.clone(),
        updated_at: now,
        enabled_mod_ids: ids,
    };
    profiles_store::save_profile(&profile)?;
    Ok(profile)
}

#[tauri::command]
pub fn rename_profile(id: String, name: String) -> AppResult<Profile> {
    let mut profile = profiles_store::load_profile_by_id(&id)?;
    profile.name = name;
    profile.updated_at = now_iso();
    profiles_store::save_profile(&profile)?;
    Ok(profile)
}

#[tauri::command]
pub fn delete_profile(id: String) -> AppResult<()> {
    if id == "default" {
        return Err(AppError::new(
            "profile_delete_forbidden",
            "不能删除 default profile",
        ));
    }
    profiles_store::delete_profile(&id)
}

#[tauri::command]
pub fn apply_profile(id: String) -> AppResult<ApplyReport> {
    let profile = profiles_store::load_profile_by_id(&id)?;
    let settings = settings::load_settings()?;
    let paths = game::resolve_paths(&settings)?;
    let entries = scan::scan_mods(&paths.mods_path)?;
    let report = profiles::apply_profile(&paths.mods_path, &profile, &entries)?;

    let mut updated = settings;
    updated.last_profile_id = Some(profile.id);
    settings::save_settings(&updated)?;

    Ok(report)
}

#[tauri::command]
pub fn snapshot_current_as_profile(name: String) -> AppResult<Profile> {
    let mods = try_resolve_mods_path()?;
    let ids = enabled_ids_from_scan(&mods)?;
    create_profile(name, Some(ids))
}
