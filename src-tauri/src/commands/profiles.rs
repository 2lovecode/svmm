use crate::domain::game;
use crate::domain::library::{self, ProfileState};
use crate::domain::profiles::{ApplyReport, Profile};
use crate::error::{AppError, AppResult};
use crate::storage::log_util::log_result;
use crate::storage::paths::{apply_marker_path, library_dir, userdata_dir};
use crate::storage::profiles_store;
use crate::storage::settings;

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn prepare() -> AppResult<()> {
    let _ = library::resume_incomplete_apply(&apply_marker_path())?;
    let settings = settings::load_settings().ok();
    if let Some(settings) = settings {
        if let Ok(resolved) = game::resolve_paths(&settings) {
            return library::migrate_installed_mods(
                &resolved.mods_path,
                &library_dir(),
                &userdata_dir(),
                &profiles_store::profiles_dir(),
                &now_iso(),
            );
        }
    }
    profiles_store::ensure_default_profile(
        &profiles_store::profiles_dir(),
        Vec::new(),
        &now_iso(),
    )?;
    Ok(())
}

/// List profiles. Creates the undeletable default group when none exist.
#[tauri::command]
pub fn list_profiles() -> AppResult<Vec<Profile>> {
    log_result((|| {
        prepare()?;
        profiles_store::list_profiles_from(&profiles_store::profiles_dir())
    })())
}

#[tauri::command]
pub fn create_profile(name: String, mod_ids: Option<Vec<String>>) -> AppResult<Profile> {
    log_result((|| {
        prepare()?;
        let ids = mod_ids.unwrap_or_default();
        for id in &ids {
            library::load_mod(&library_dir(), id)?;
        }
        let now = now_iso();
        let profile = Profile {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            created_at: now.clone(),
            updated_at: now,
            mod_ids: ids,
        };
        profiles_store::save_profile(&profile)?;
        Ok(profile)
    })())
}

#[tauri::command]
pub fn rename_profile(id: String, name: String) -> AppResult<Profile> {
    log_result((|| {
        let mut profile = profiles_store::load_profile_by_id(&id)?;
        profile.name = name;
        profile.updated_at = now_iso();
        profiles_store::save_profile(&profile)?;
        Ok(profile)
    })())
}

#[tauri::command]
pub fn delete_profile(id: String) -> AppResult<()> {
    log_result((|| {
        if id == "default" {
            return Err(AppError::new(
                "profile_delete_forbidden",
                "不能删除默认方案",
            ));
        }
        profiles_store::delete_profile(&id)
    })())
}

#[tauri::command]
pub fn add_profile_mod(id: String, mod_id: String) -> AppResult<Profile> {
    log_result((|| {
        library::load_mod(&library_dir(), &mod_id)?;
        let mut profile = profiles_store::load_profile_by_id(&id)?;
        if !profile.mod_ids.iter().any(|item| item == &mod_id) {
            profile.mod_ids.push(mod_id);
        }
        profile.updated_at = now_iso();
        profiles_store::save_profile(&profile)?;
        Ok(profile)
    })())
}

#[tauri::command]
pub fn remove_profile_mod(id: String, mod_id: String) -> AppResult<Profile> {
    log_result((|| {
        let mut profile = profiles_store::load_profile_by_id(&id)?;
        profile.mod_ids.retain(|item| item != &mod_id);
        profile.updated_at = now_iso();
        profiles_store::save_profile(&profile)?;
        Ok(profile)
    })())
}

#[tauri::command]
pub fn apply_profile(id: String) -> AppResult<ApplyReport> {
    log_result((|| {
        prepare()?;
        let profile = profiles_store::load_profile_by_id(&id)?;
        let settings = settings::load_settings()?;
        let paths = game::resolve_paths(&settings)?;
        let report = library::apply_profile(
            &paths.mods_path,
            &library_dir(),
            &userdata_dir(),
            &apply_marker_path(),
            &profile,
            false,
        )?;
        let mut updated = settings;
        updated.last_profile_id = Some(profile.id);
        settings::save_settings(&updated)?;
        Ok(report)
    })())
}

#[tauri::command]
pub fn profile_detail(id: String) -> AppResult<ProfileState> {
    log_result((|| {
        prepare()?;
        let profile = profiles_store::load_profile_by_id(&id)?;
        let mods = settings::load_settings()
            .ok()
            .and_then(|s| game::resolve_paths(&s).ok())
            .map(|p| p.mods_path);
        library::profile_state(mods.as_deref(), &library_dir(), &profile)
    })())
}

#[tauri::command]
pub fn snapshot_current_as_profile(name: String) -> AppResult<Profile> {
    log_result((|| {
        prepare()?;
        let settings = settings::load_settings()?;
        let paths = game::resolve_paths(&settings)?;
        let entries = crate::domain::mods::scan::scan_mods(&paths.mods_path)?;
        let ids: Vec<String> = entries
            .into_iter()
            .filter(|e| e.enabled && !library::is_smapi_bundled(&e.folder_path, Some(&e.id)))
            .map(|e| e.id)
            .filter(|id| library::load_mod(&library_dir(), id).is_ok())
            .collect();
        create_profile(name, Some(ids))
    })())
}
