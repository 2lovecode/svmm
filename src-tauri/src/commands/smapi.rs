use crate::domain::game;
use crate::domain::smapi;
use crate::error::AppResult;
use crate::storage::log_util::log_result;
use crate::storage::settings;

/// Launch SMAPI using paths resolved from settings (no UI args).
#[tauri::command]
pub fn launch_smapi() -> AppResult<()> {
    log_result((|| {
        let settings = settings::load_settings()?;
        let paths = game::resolve_paths(&settings)?;
        smapi::launch_smapi(&paths.smapi_path, &paths.game_path)
    })())
}
