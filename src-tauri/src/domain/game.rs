use std::env;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::storage::settings::Settings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GamePaths {
    pub game_path: PathBuf,
    pub smapi_path: PathBuf,
    pub mods_path: PathBuf,
}

impl GamePaths {
    pub fn from_game_dir(game_path: PathBuf) -> Self {
        let smapi_path = game_path.join("StardewModdingAPI.exe");
        let mods_path = game_path.join("Mods");
        Self {
            game_path,
            smapi_path,
            mods_path,
        }
    }
}

/// Build ordered install candidates from explicit Windows roots.
/// Order locked by brief: Steam (x86) → Steam ProgramFiles → Xbox → GOG.
fn candidate_game_dirs_from(
    program_files_x86: Option<&Path>,
    program_files: Option<&Path>,
    local_app_data: Option<&Path>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(pf86) = program_files_x86 {
        candidates.push(
            pf86.join("Steam")
                .join("steamapps")
                .join("common")
                .join("Stardew Valley"),
        );
    }

    if let Some(pf) = program_files {
        candidates.push(
            pf.join("Steam")
                .join("steamapps")
                .join("common")
                .join("Stardew Valley"),
        );
    }

    if let Some(local) = local_app_data {
        candidates.push(
            local
                .join("XboxGames")
                .join("Stardew Valley")
                .join("Content"),
        );
    }

    if let Some(pf86) = program_files_x86 {
        candidates.push(
            pf86.join("GOG Galaxy")
                .join("Games")
                .join("Stardew Valley"),
        );
    }

    candidates
}

fn candidate_game_dirs() -> Vec<PathBuf> {
    let pf86 = env::var_os("ProgramFiles(x86)").map(PathBuf::from);
    let pf = env::var_os("ProgramFiles").map(PathBuf::from);
    let local = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    candidate_game_dirs_from(
        pf86.as_deref(),
        pf.as_deref(),
        local.as_deref(),
    )
}

pub fn discover_game_paths() -> AppResult<Option<GamePaths>> {
    for dir in candidate_game_dirs() {
        if dir.is_dir() {
            return Ok(Some(GamePaths::from_game_dir(dir)));
        }
    }
    Ok(None)
}

pub fn resolve_paths(settings: &Settings) -> AppResult<GamePaths> {
    let discovered = discover_game_paths()?;

    let game_path = settings
        .game_path
        .clone()
        .or_else(|| discovered.as_ref().map(|d| d.game_path.clone()))
        .ok_or_else(|| {
            AppError::new(
                "game_path_not_found",
                "未找到星露谷物语安装目录，请手动设置路径",
            )
        })?;

    let smapi_path = settings
        .smapi_path
        .clone()
        .unwrap_or_else(|| game_path.join("StardewModdingAPI.exe"));

    let mods_path = settings
        .mods_path
        .clone()
        .unwrap_or_else(|| game_path.join("Mods"));

    Ok(GamePaths {
        game_path,
        smapi_path,
        mods_path,
    })
}

pub fn validate_paths(paths: &GamePaths) -> AppResult<()> {
    if !paths.smapi_path.is_file() {
        return Err(AppError::new(
            "smapi_missing",
            "未找到 StardewModdingAPI.exe，请确认已安装 SMAPI",
        )
        .with_detail(paths.smapi_path.display().to_string()));
    }
    if !paths.mods_path.is_dir() {
        return Err(AppError::new("mods_dir_missing", "未找到 Mods 目录")
            .with_detail(paths.mods_path.display().to_string()));
    }
    if !paths.game_path.is_dir() {
        return Err(AppError::new("game_path_invalid", "游戏目录无效")
            .with_detail(paths.game_path.display().to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_game_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "svmm-game-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Mods")).unwrap();
        fs::write(root.join("StardewModdingAPI.exe"), b"").unwrap();
        root
    }

    #[test]
    fn validate_requires_smapi_and_mods() {
        let root = temp_game_root();
        let paths = GamePaths {
            game_path: root.clone(),
            smapi_path: root.join("StardewModdingAPI.exe"),
            mods_path: root.join("Mods"),
        };
        assert!(validate_paths(&paths).is_ok());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_fails_without_smapi() {
        let root = temp_game_root();
        fs::remove_file(root.join("StardewModdingAPI.exe")).unwrap();
        let paths = GamePaths::from_game_dir(root.clone());
        let err = validate_paths(&paths).unwrap_err();
        assert_eq!(err.code, "smapi_missing");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_fails_without_mods_dir() {
        let root = temp_game_root();
        fs::remove_dir_all(root.join("Mods")).unwrap();
        let paths = GamePaths::from_game_dir(root.clone());
        let err = validate_paths(&paths).unwrap_err();
        assert_eq!(err.code, "mods_dir_missing");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_prefers_manual_overrides() {
        let root = temp_game_root();
        let settings = Settings {
            game_path: Some(root.clone()),
            smapi_path: Some(root.join("custom-smapi.exe")),
            mods_path: Some(root.join("CustomMods")),
            ..Settings::default()
        };
        let paths = resolve_paths(&settings).unwrap();
        assert_eq!(paths.game_path, root);
        assert_eq!(paths.smapi_path, root.join("custom-smapi.exe"));
        assert_eq!(paths.mods_path, root.join("CustomMods"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_derives_smapi_and_mods_from_game_path() {
        let root = temp_game_root();
        let settings = Settings {
            game_path: Some(root.clone()),
            ..Settings::default()
        };
        let paths = resolve_paths(&settings).unwrap();
        assert_eq!(paths.smapi_path, root.join("StardewModdingAPI.exe"));
        assert_eq!(paths.mods_path, root.join("Mods"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn candidate_order_is_steam_x86_steam_pf_xbox_gog() {
        let pf86 = PathBuf::from(r"C:\Fake Program Files (x86)");
        let pf = PathBuf::from(r"C:\Fake Program Files");
        let local = PathBuf::from(r"C:\Fake\LocalAppData");
        let dirs = candidate_game_dirs_from(Some(&pf86), Some(&pf), Some(&local));
        let labels: Vec<&str> = dirs
            .iter()
            .map(|p| {
                let s = p.to_string_lossy();
                if s.contains("XboxGames") {
                    "xbox"
                } else if s.contains("GOG Galaxy") {
                    "gog"
                } else if s.contains("Steam") {
                    if s.contains("Program Files (x86)") {
                        "steam_x86"
                    } else {
                        "steam_pf"
                    }
                } else {
                    "other"
                }
            })
            .collect();
        assert_eq!(labels, vec!["steam_x86", "steam_pf", "xbox", "gog"]);
    }
}
