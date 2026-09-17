use serde::Serialize;

use crate::domain::game;
use crate::domain::mods::scan;
use crate::domain::nexus::{self, NexusUser};
use crate::error::{AppError, AppResult};
use crate::storage::{secure_key, settings};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NexusStatus {
    pub has_key: bool,
}

#[tauri::command]
pub fn nexus_set_key(key: String) -> AppResult<()> {
    secure_key::set_nexus_api_key(&key)
}

#[tauri::command]
pub fn nexus_clear_key() -> AppResult<()> {
    secure_key::clear_nexus_api_key()
}

#[tauri::command]
pub fn nexus_status() -> AppResult<NexusStatus> {
    Ok(NexusStatus {
        has_key: secure_key::has_nexus_api_key(),
    })
}

#[tauri::command]
pub fn nexus_validate() -> AppResult<NexusUser> {
    nexus::validate_nexus_api_key()
}

#[tauri::command]
pub fn nexus_endorse(mod_id: u32, version: Option<String>) -> AppResult<()> {
    nexus::endorse_mod(mod_id, version.as_deref())
}

#[tauri::command]
pub fn nexus_update_mod(folder_path: String) -> AppResult<scan::ModEntry> {
    let settings = settings::load_settings()?;
    let paths = game::resolve_paths(&settings)?;
    let mods = scan::scan_mods(&paths.mods_path)?;
    let entry = mods
        .into_iter()
        .find(|m| m.folder_path == folder_path)
        .ok_or_else(|| {
            AppError::new("mod_not_found", "未找到指定模组").with_detail(folder_path)
        })?;
    nexus::update_mod_from_nexus(&entry, &paths.mods_path)
}
