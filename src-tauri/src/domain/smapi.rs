use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{AppError, AppResult};

/// Program path and working directory used to launch SMAPI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmapiLaunchSpec {
    pub program: PathBuf,
    pub current_dir: PathBuf,
}

/// Pure builder for the SMAPI process (no spawn). Used by tests and `launch_smapi`.
pub fn smapi_launch_spec(smapi_path: &Path, game_path: &Path) -> SmapiLaunchSpec {
    SmapiLaunchSpec {
        program: smapi_path.to_path_buf(),
        current_dir: game_path.to_path_buf(),
    }
}

fn build_smapi_command(spec: &SmapiLaunchSpec) -> Command {
    let mut cmd = Command::new(&spec.program);
    cmd.current_dir(&spec.current_dir);
    cmd.env("SteamAppId", "413150");
    cmd.env("SteamGameId", "413150");
    cmd
}

fn path_is_steam_install(game_path: &Path) -> bool {
    game_path.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("steamapps")
    })
}

/// macOS Steam copies call RestartAppIfNecessary and start a second process
/// unless the game was opened by the Steam client.
fn should_launch_via_steam(game_path: &Path) -> bool {
    cfg!(target_os = "macos") && path_is_steam_install(game_path)
}

const STEAM_RUN_URL: &str = "steam://rungameid/413150";

/// Spawn SMAPI. On a macOS Steam install, ask Steam to start the game so only one copy runs.
pub fn launch_smapi(smapi_path: &Path, game_path: &Path) -> AppResult<()> {
    if should_launch_via_steam(game_path) {
        Command::new("open")
            .arg(STEAM_RUN_URL)
            .spawn()
            .map_err(|e| AppError::new("launch_failed", e.to_string()))?;
        return Ok(());
    }

    let spec = smapi_launch_spec(smapi_path, game_path);
    build_smapi_command(&spec)
        .spawn()
        .map_err(|e| AppError::new("launch_failed", e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_spec_sets_program_and_cwd() {
        let smapi = PathBuf::from(r"C:\Games\Stardew Valley\StardewModdingAPI.exe");
        let game = PathBuf::from(r"C:\Games\Stardew Valley");
        let spec = smapi_launch_spec(&smapi, &game);
        assert_eq!(spec.program, smapi);
        assert_eq!(spec.current_dir, game);
    }

    #[test]
    fn build_command_uses_spec_paths() {
        let smapi = PathBuf::from("StardewModdingAPI.exe");
        let game = PathBuf::from(".");
        let spec = smapi_launch_spec(&smapi, &game);
        let cmd = build_smapi_command(&spec);
        // Command has no stable getters; assert via Debug which includes program + cwd.
        let debug = format!("{cmd:?}");
        assert!(
            debug.contains("StardewModdingAPI.exe"),
            "expected program in Command debug: {debug}"
        );
        assert!(
            debug.contains("current_dir") || debug.contains('.'),
            "expected cwd in Command debug: {debug}"
        );
        assert!(
            debug.contains("SteamAppId") && debug.contains("413150"),
            "expected Steam app id so Steam does not relaunch the game: {debug}"
        );
    }

    #[test]
    fn steamapps_path_is_a_steam_install() {
        let steam = PathBuf::from("/Users/lh/Library/Application Support/Steam/steamapps/common/Stardew Valley/Contents/MacOS");
        let gog = PathBuf::from("/Games/Stardew Valley");
        assert!(path_is_steam_install(&steam));
        assert!(!path_is_steam_install(&gog));
        assert_eq!(should_launch_via_steam(&steam), cfg!(target_os = "macos"));
        assert!(!should_launch_via_steam(&gog));
    }
}
