use crate::domain::game::{self, GamePaths};
use crate::error::AppResult;
use crate::storage::settings::{self, Settings};

#[tauri::command]
pub fn get_settings() -> AppResult<Settings> {
    settings::load_settings()
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> AppResult<()> {
    settings::save_settings(&settings)
}

#[tauri::command]
pub fn discover_paths() -> AppResult<Option<GamePaths>> {
    game::discover_game_paths()
}

#[tauri::command]
pub fn validate_paths(paths: GamePaths) -> AppResult<()> {
    game::validate_paths(&paths)
}
