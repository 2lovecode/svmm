use crate::domain::game::{self, GamePaths};
use crate::domain::http;
use crate::error::AppResult;
use crate::storage::log_util::log_result;
use crate::storage::settings::{self, Settings};

#[tauri::command]
pub fn get_settings() -> AppResult<Settings> {
    log_result(settings::load_settings())
}

#[tauri::command]
pub fn save_settings(mut settings: Settings) -> AppResult<()> {
    if let Some(path) = settings.game_path.take() {
        settings.game_path = Some(game::normalize_game_dir(path));
    }
    settings.download_proxy = http::normalize_download_proxy(settings.download_proxy.as_deref())?;
    log_result(settings::save_settings(&settings))
}

#[tauri::command]
pub fn discover_paths() -> AppResult<Option<GamePaths>> {
    log_result(game::discover_game_paths())
}

#[tauri::command]
pub fn validate_paths(paths: GamePaths) -> AppResult<()> {
    log_result(game::validate_paths(&paths))
}
