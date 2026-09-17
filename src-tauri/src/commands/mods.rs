use crate::domain::game;
use crate::domain::mods::enable;
use crate::domain::mods::install;
use crate::domain::mods::scan::{self, ModEntry};
use crate::domain::nexus;
use crate::error::{AppError, AppResult};
use crate::storage::log_util::log_result;
use crate::storage::settings;

fn join_blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> impl std::future::Future<Output = AppResult<T>> {
    async move {
        tauri::async_runtime::spawn_blocking(f)
            .await
            .map_err(|_| AppError::new("task_join_failed", "后台任务失败"))?
    }
}

#[tauri::command]
pub fn scan_mods() -> AppResult<Vec<ModEntry>> {
    log_result((|| {
        let settings = settings::load_settings()?;
        let paths = game::resolve_paths(&settings)?;
        scan::scan_mods(&paths.mods_path)
    })())
}

#[tauri::command]
pub fn set_mod_enabled(folder_path: String, enabled: bool) -> AppResult<ModEntry> {
    log_result((|| {
        let settings = settings::load_settings()?;
        let paths = game::resolve_paths(&settings)?;
        enable::set_mod_enabled(&paths.mods_path, &folder_path, enabled)
    })())
}

pub fn install_mod_zip_blocking(path: String) -> AppResult<ModEntry> {
    log_result((|| {
        let settings = settings::load_settings()?;
        let paths = game::resolve_paths(&settings)?;
        install::install_mod_zip(std::path::Path::new(&path), &paths.mods_path)
    })())
}

#[tauri::command]
pub async fn install_mod_zip(path: String) -> AppResult<ModEntry> {
    join_blocking(move || install_mod_zip_blocking(path)).await
}

/// Shared by the Tauri command and deep-link handler.
pub fn install_from_nxm_blocking(url: String) -> AppResult<ModEntry> {
    log_result((|| {
        let settings = settings::load_settings()?;
        let paths = game::resolve_paths(&settings)?;
        nexus::handle_nxm_url(&url, &paths.mods_path)
    })())
}

#[tauri::command]
pub async fn install_from_nxm(url: String) -> AppResult<ModEntry> {
    join_blocking(move || install_from_nxm_blocking(url)).await
}
