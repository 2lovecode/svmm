use crate::domain::game;
use crate::domain::mods::enable;
use crate::domain::mods::install;
use crate::domain::mods::scan::{self, ModEntry};
use crate::domain::nexus;
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

#[tauri::command]
pub fn install_mod_zip(path: String) -> AppResult<ModEntry> {
    let settings = settings::load_settings()?;
    let paths = game::resolve_paths(&settings)?;
    install::install_mod_zip(std::path::Path::new(&path), &paths.mods_path)
}

#[tauri::command]
pub fn install_from_nxm(url: String) -> AppResult<ModEntry> {
    let settings = settings::load_settings()?;
    let paths = game::resolve_paths(&settings)?;
    nexus::handle_nxm_url(&url, &paths.mods_path)
}
