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

fn should_launch_via_steam(game_path: &Path) -> bool {
    path_is_steam_install(game_path)
}

const STEAM_RUN_URL: &str = "steam://rungameid/413150";
const STEAM_APP_ID: &str = "413150";

fn steam_install_root(game_path: &Path) -> Option<PathBuf> {
    let mut current = Some(game_path);
    while let Some(path) = current {
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("steamapps"))
        {
            return path.parent().map(Path::to_path_buf);
        }
        current = path.parent();
    }
    None
}

fn vdf_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn vdf_unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn smapi_launch_options_value(smapi_path: &Path) -> String {
    vdf_escape(&format!("\"{}\" %command%", smapi_path.display()))
}

fn options_load_smapi(escaped: &str) -> bool {
    vdf_unescape(escaped)
        .to_ascii_lowercase()
        .contains("stardewmoddingapi")
}

fn matching_brace(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in text[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + index + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_stardew_app_block(text: &str) -> Option<(usize, usize)> {
    let marker = format!("\"{STEAM_APP_ID}\"");
    let mut search = 0;
    while let Some(relative) = text[search..].find(&marker) {
        let start = search + relative;
        let after = start + marker.len();
        let Some(brace_rel) = text[after..].find('{') else {
            return None;
        };
        if !text[after..after + brace_rel].trim().is_empty() {
            search = after;
            continue;
        }
        let open = after + brace_rel;
        let Some(end) = matching_brace(text, open) else {
            return None;
        };
        let block = &text[start..end];
        if block.contains("\"LastPlayed\"") || block.contains("\"Playtime\"") {
            return Some((start, end));
        }
        search = end;
    }
    None
}

fn skip_vdf_string(text: &str, quote_at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(quote_at) != Some(&b'"') {
        return None;
    }
    let mut index = quote_at + 1;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += 2;
            continue;
        }
        if bytes[index] == b'"' {
            return Some(index + 1);
        }
        index += 1;
    }
    None
}

/// Returns the file text and whether it changed. Existing SMAPI launch options are left alone.
fn ensure_smapi_launch_options(text: &str, smapi_path: &Path) -> AppResult<(String, bool)> {
    let (start, end) = find_stardew_app_block(text).ok_or_else(|| {
        AppError::new(
            "steam_launch_options",
            "Steam 配置里没有星露谷物语。请先在 Steam 里启动一次游戏。",
        )
    })?;
    let block = &text[start..end];
    let desired = smapi_launch_options_value(smapi_path);
    if let Some(key_at) = block.find("\"LaunchOptions\"") {
        let after_key = key_at + "\"LaunchOptions\"".len();
        let Some(quote_rel) = block[after_key..].find('"') else {
            return Err(AppError::new(
                "steam_launch_options",
                "Steam 启动项格式无法识别",
            ));
        };
        let quote_at = after_key + quote_rel;
        let Some(value_end) = skip_vdf_string(block, quote_at) else {
            return Err(AppError::new(
                "steam_launch_options",
                "Steam 启动项格式无法识别",
            ));
        };
        let escaped = &block[quote_at + 1..value_end - 1];
        if options_load_smapi(escaped) {
            return Ok((text.to_string(), false));
        }
        let mut next = String::with_capacity(text.len() + desired.len());
        next.push_str(&text[..start + quote_at + 1]);
        next.push_str(&desired);
        next.push_str(&text[start + value_end - 1..]);
        return Ok((next, true));
    }

    let insert_at = end - 1;
    let indent = block
        .lines()
        .find_map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("\"LastPlayed\"") || trimmed.starts_with("\"Playtime\"") {
                Some(&line[..line.len() - trimmed.len()])
            } else {
                None
            }
        })
        .unwrap_or("\t\t\t\t\t\t");
    let line = format!("\n{indent}\"LaunchOptions\"\t\t\"{desired}\"");
    let mut next = String::with_capacity(text.len() + line.len());
    next.push_str(&text[..insert_at]);
    next.push_str(&line);
    next.push_str(&text[insert_at..]);
    Ok((next, true))
}

fn write_windows_launch_options(game_path: &Path, smapi_path: &Path) -> AppResult<()> {
    let steam_root = steam_install_root(game_path).ok_or_else(|| {
        AppError::new("steam_launch_options", "找不到 Steam 安装目录")
    })?;
    let userdata = steam_root.join("userdata");
    let entries = fs_read_dir(&userdata)?;
    let mut saw_config = false;
    for user_dir in entries {
        let config = user_dir.join("config").join("localconfig.vdf");
        if !config.is_file() {
            continue;
        }
        saw_config = true;
        let original = std::fs::read_to_string(&config).map_err(|err| {
            AppError::new("steam_launch_options", "无法读取 Steam 配置")
                .with_detail(err.to_string())
        })?;
        let (next, changed) = match ensure_smapi_launch_options(&original, smapi_path) {
            Ok(pair) => pair,
            Err(err) if err.code == "steam_launch_options" => continue,
            Err(err) => return Err(err),
        };
        if changed {
            std::fs::write(&config, next).map_err(|err| {
                AppError::new("steam_launch_options", "无法写入 Steam 启动项")
                    .with_detail(err.to_string())
            })?;
        }
        return Ok(());
    }
    if saw_config {
        return Err(AppError::new(
            "steam_launch_options",
            "Steam 配置里没有星露谷物语。请先在 Steam 里启动一次游戏。",
        ));
    }
    Err(AppError::new(
        "steam_launch_options",
        "找不到 Steam 用户配置，无法让 Steam 加载 SMAPI",
    ))
}

fn fs_read_dir(path: &Path) -> AppResult<Vec<PathBuf>> {
    let entries = std::fs::read_dir(path).map_err(|err| {
        AppError::new("steam_launch_options", "找不到 Steam 用户目录")
            .with_detail(err.to_string())
    })?;
    let mut dirs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| {
            AppError::new("steam_launch_options", "无法读取 Steam 用户目录")
                .with_detail(err.to_string())
        })?;
        if entry.path().is_dir() {
            dirs.push(entry.path());
        }
    }
    Ok(dirs)
}

fn open_steam_protocol() -> AppResult<()> {
    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", "start", "", STEAM_RUN_URL])
            .spawn()
            .map_err(|err| AppError::new("launch_failed", err.to_string()))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(STEAM_RUN_URL)
            .spawn()
            .map_err(|err| AppError::new("launch_failed", err.to_string()))?;
        return Ok(());
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(STEAM_RUN_URL)
            .spawn()
            .map_err(|err| AppError::new("launch_failed", err.to_string()))?;
        Ok(())
    }
}

/// Spawn SMAPI. Steam copies are started through Steam so achievements stay attached.
/// On Windows that also writes the SMAPI launch option when it is missing.
pub fn launch_smapi(smapi_path: &Path, game_path: &Path) -> AppResult<()> {
    if should_launch_via_steam(game_path) {
        #[cfg(windows)]
        write_windows_launch_options(game_path, smapi_path)?;
        return open_steam_protocol();
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
    }

    #[test]
    fn steamapps_path_is_a_steam_install() {
        let steam = PathBuf::from("/Users/lh/Library/Application Support/Steam/steamapps/common/Stardew Valley/Contents/MacOS");
        let gog = PathBuf::from("/Games/Stardew Valley");
        assert!(path_is_steam_install(&steam));
        assert!(!path_is_steam_install(&gog));
        assert!(should_launch_via_steam(&steam));
        assert!(!should_launch_via_steam(&gog));
    }

    #[test]
    fn keeps_existing_smapi_launch_options() {
        let text = r#"
"413150"
{
    "LastPlayed"		"1"
    "LaunchOptions"		"\"D:\\Games\\StardewModdingAPI.exe\" %command%"
}
"#;
        let smapi = PathBuf::from(r"D:\Games\StardewModdingAPI.exe");
        let (next, changed) = ensure_smapi_launch_options(text, &smapi).unwrap();
        assert!(!changed);
        assert_eq!(next, text);
    }

    #[test]
    fn inserts_smapi_launch_options_when_missing() {
        let text = "\"413150\"\n{\n\t\"LastPlayed\"\t\t\"1\"\n}\n";
        let smapi = PathBuf::from(r"D:\Games\StardewModdingAPI.exe");
        let (next, changed) = ensure_smapi_launch_options(text, &smapi).unwrap();
        assert!(changed);
        assert!(next.contains("StardewModdingAPI.exe"));
        assert!(next.contains("%command%"));
        assert!(options_load_smapi(
            &smapi_launch_options_value(&smapi)
        ));
    }

    #[test]
    fn replaces_launch_options_that_skip_smapi() {
        let text = r#"
"413150"
{
    "Playtime"		"1"
    "LaunchOptions"		"-foo"
}
"#;
        let smapi = PathBuf::from(r"D:\Games\StardewModdingAPI.exe");
        let (next, changed) = ensure_smapi_launch_options(text, &smapi).unwrap();
        assert!(changed);
        assert!(next.contains("StardewModdingAPI.exe"));
        assert!(!next.contains("-foo"));
    }
}
