use crate::domain::game;
use crate::domain::mods::enable;
use crate::domain::mods::scan::{self, ModEntry};
use crate::error::AppResult;
use crate::storage::settings;

#[tauri::command]
pub fn scan_mods() -> AppResult<Vec<ModEntry>> {
    let settings = settings::load_settings()?;
    let paths = game::resolve_paths(&settings)?;
    scan::scan_mods(&paths.mods_path)
}

#[tauri::command]
pub fn set_mod_enabled(folder_path: String, enabled: bool) -> AppResult<ModEntry> {
    let settings = settings::load_settings()?;
    let paths = game::resolve_paths(&settings)?;
    enable::set_mod_enabled(&paths.mods_path, &folder_path, enabled)
}
