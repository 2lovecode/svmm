use crate::domain::game;
use crate::domain::mods::scan;
use crate::domain::smapi_update::{self, UpdateInfo};
use crate::error::AppResult;
use crate::storage::log_util::log_result;
use crate::storage::settings;

#[tauri::command]
pub fn check_mod_updates() -> AppResult<Vec<UpdateInfo>> {
    log_result((|| {
        let settings = settings::load_settings()?;
        let paths = game::resolve_paths(&settings)?;
        let mods = scan::scan_mods(&paths.mods_path)?;
        smapi_update::check_updates(&mods)
    })())
}
