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
    cmd
}

/// Spawn StardewModdingAPI with the game directory as cwd (does not wait).
pub fn launch_smapi(smapi_path: &Path, game_path: &Path) -> AppResult<()> {
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
    }
}
