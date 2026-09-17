use crate::domain::game;
use crate::domain::mods::scan;
use crate::domain::smapi_update::{self, UpdateInfo};
use crate::error::{AppError, AppResult};
use crate::storage::log_util::log_result;
use crate::storage::settings;

fn check_mod_updates_blocking() -> AppResult<Vec<UpdateInfo>> {
    log_result((|| {
        let settings = settings::load_settings()?;
        let paths = game::resolve_paths(&settings)?;
        let mods = scan::scan_mods(&paths.mods_path)?;
        smapi_update::check_updates(&mods)
    })())
}

#[tauri::command]
pub async fn check_mod_updates() -> AppResult<Vec<UpdateInfo>> {
    tauri::async_runtime::spawn_blocking(check_mod_updates_blocking)
        .await
        .map_err(|_| AppError::new("task_join_failed", "后台任务失败"))?
}
