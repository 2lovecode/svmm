use std::path::PathBuf;

use serde::Serialize;

use tauri::Emitter;

use crate::domain::game;
use crate::domain::smapi;
use crate::domain::smapi_install;
use crate::error::{AppError, AppResult};
use crate::storage::log_util::log_result;
use crate::storage::settings;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmapiStatus {
    pub installed: bool,
    pub game_found: bool,
    pub game_path: Option<PathBuf>,
    pub smapi_path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmapiInstallReport {
    pub version: String,
    pub smapi_path: PathBuf,
}

#[tauri::command]
pub fn smapi_status() -> AppResult<SmapiStatus> {
    let settings = settings::load_settings()?;
    match game::resolve_paths(&settings) {
        Ok(paths) => Ok(SmapiStatus {
            installed: paths.smapi_path.is_file(),
            game_found: true,
            game_path: Some(paths.game_path),
            smapi_path: Some(paths.smapi_path),
        }),
        Err(err) if err.code == "game_path_not_found" => Ok(SmapiStatus {
            installed: false,
            game_found: false,
            game_path: None,
            smapi_path: None,
        }),
        Err(err) => Err(err),
    }
}

/// Launch SMAPI using paths resolved from settings (no UI args).
#[tauri::command]
pub fn launch_smapi() -> AppResult<()> {
    log_result((|| {
        let settings = settings::load_settings()?;
        let paths = game::resolve_paths(&settings)?;
        smapi::launch_smapi(&paths.smapi_path, &paths.game_path)
    })())
}

fn install_smapi_blocking(app: tauri::AppHandle) -> AppResult<SmapiInstallReport> {
    log_result((|| {
        let settings = settings::load_settings()?;
        let paths = game::resolve_paths(&settings)?;
        let version = smapi_install::install_into(&paths.game_path, &|progress| {
            let _ = app.emit("smapi-install-progress", &progress);
        })?;
        if settings
            .smapi_path
            .as_ref()
            .is_some_and(|path| !path.is_file())
        {
            let mut next = settings;
            next.smapi_path = None;
            settings::save_settings(&next)?;
        }
        Ok(SmapiInstallReport {
            version,
            smapi_path: paths.game_path.join(game::smapi_file_name()),
        })
    })())
}

#[tauri::command]
pub async fn install_smapi(app: tauri::AppHandle) -> AppResult<SmapiInstallReport> {
    match tauri::async_runtime::spawn_blocking(move || install_smapi_blocking(app)).await {
        Ok(result) => result,
        Err(_) => Err(AppError::new("task_join_failed", "后台任务失败")),
    }
}
