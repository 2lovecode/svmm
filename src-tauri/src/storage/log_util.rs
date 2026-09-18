use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use crate::error::{AppError, AppResult};
use crate::storage::paths::app_data_dir;

pub fn logs_dir() -> PathBuf {
    app_data_dir().join("logs")
}

pub fn log_file_path() -> PathBuf {
    logs_dir().join("svmm.log")
}

pub fn ensure_logs_dir() -> AppResult<PathBuf> {
    let dir = logs_dir();
    fs::create_dir_all(&dir).map_err(|e| {
        AppError::new("log_dir_failed", "无法创建日志目录").with_detail(e.to_string())
    })?;
    Ok(dir)
}

/// Redact common secret-bearing fragments before writing to disk.
fn sanitize_for_log(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let markers = [
        "key=",
        "apikey=",
        "api_key=",
        "api-key=",
        "authorization:",
        "bearer ",
    ];
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        let mut matched: Option<&str> = None;
        for marker in markers {
            if lower[i..].starts_with(marker) {
                matched = Some(marker);
                break;
            }
        }
        if let Some(marker) = matched {
            out.push_str(&text[i..i + marker.len()]);
            i += marker.len();
            out.push_str("***");
            while i < text.len() {
                let ch = text[i..].chars().next().unwrap();
                if ch.is_whitespace() || ch == '&' || ch == '"' || ch == '\'' {
                    break;
                }
                i += ch.len_utf8();
            }
        } else {
            let ch = text[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

pub fn append_log(line: &str) -> AppResult<()> {
    ensure_logs_dir()?;
    let path = log_file_path();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| {
            AppError::new("log_write_failed", "无法写入日志文件").with_detail(e.to_string())
        })?;
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let safe = sanitize_for_log(line);
    writeln!(file, "[{ts}] {safe}").map_err(|e| {
        AppError::new("log_write_failed", "无法写入日志文件").with_detail(e.to_string())
    })?;
    Ok(())
}

pub fn log_app_error(err: &AppError) {
    let mut line = format!("{}: {}", err.code, err.message);
    if let Some(detail) = err.detail.as_deref() {
        if !detail.is_empty() {
            line.push_str(" — ");
            line.push_str(detail);
        }
    }
    let _ = append_log(&line);
}

pub fn log_result<T>(result: AppResult<T>) -> AppResult<T> {
    if let Err(ref e) = result {
        log_app_error(e);
    }
    result
}

/// Ensure the logs folder exists and open it in the system file manager.
#[tauri::command]
pub fn open_log_dir() -> AppResult<()> {
    let dir = ensure_logs_dir()?;
    // Ensure the log file exists so the folder is clearly SVMM's log location.
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path());

    open_dir_in_file_manager(&dir)
}

fn open_dir_in_file_manager(dir: &std::path::Path) -> AppResult<()> {
    #[cfg(windows)]
    {
        Command::new("explorer").arg(dir).spawn().map_err(|e| {
            AppError::new("open_log_dir_failed", "无法打开日志目录").with_detail(e.to_string())
        })?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(dir).spawn().map_err(|e| {
            AppError::new("open_log_dir_failed", "无法打开日志目录").with_detail(e.to_string())
        })?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(dir).spawn().map_err(|e| {
            AppError::new("open_log_dir_failed", "无法打开日志目录").with_detail(e.to_string())
        })?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    {
        let _ = dir;
        Err(AppError::new(
            "open_log_dir_unsupported",
            "当前平台不支持打开日志目录",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_redacts_key_query_param() {
        let s = sanitize_for_log("nxm://stardewvalley/mods/1/2?key=SECRET123&expires=1");
        assert!(s.contains("key=***"));
        assert!(!s.contains("SECRET123"));
    }

    #[test]
    fn sanitize_redacts_bearer_token() {
        let s = sanitize_for_log("Authorization: Bearer abcdef.token.value");
        assert!(s.to_ascii_lowercase().contains("bearer ***"));
        assert!(!s.contains("abcdef.token.value"));
    }
}
