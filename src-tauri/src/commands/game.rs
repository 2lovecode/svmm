use crate::domain::game::{self, GamePaths};
use crate::error::AppResult;
use crate::storage::log_util::log_result;
use crate::storage::settings::{self, Settings};

#[tauri::command]
pub fn get_settings() -> AppResult<Settings> {
    log_result(settings::load_settings())
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> AppResult<()> {
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
