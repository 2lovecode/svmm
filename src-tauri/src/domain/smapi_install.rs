use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::domain::game::smapi_file_name;
use crate::domain::http::{self, download_percent};
use crate::domain::mods::install::validate_zip_entry_path;
use crate::error::{AppError, AppResult};

const RELEASES_URL: &str = "https://api.github.com/repos/Pathoschild/SMAPI/releases/latest";
/// HTML release page. Used when the API returns 403/429 (shared proxy IPs hit the
/// unauthenticated 60/hour limit, while this redirect does not).
const RELEASES_PAGE: &str = "https://github.com/Pathoschild/SMAPI/releases/latest";
const REQUEST_TIMEOUT_SECS: u64 = 180;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct InstallerAsset {
    pub name: String,
    #[serde(rename = "browser_download_url")]
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub phase: String,
    pub message: String,
    pub received: u64,
    pub total: Option<u64>,
    pub percent: Option<u8>,
}

impl InstallProgress {
    fn step(phase: &str, message: impl Into<String>) -> Self {
        Self {
            phase: phase.to_string(),
            message: message.into(),
            received: 0,
            total: None,
            percent: None,
        }
    }

    fn download(version: &str, received: u64, total: Option<u64>) -> Self {
        let percent = download_percent(received, total);
        let message = match total {
            Some(total) => format!(
                "正在下载 SMAPI {version}（{} / {}）",
                format_bytes(received),
                format_bytes(total)
            ),
            None => format!("正在下载 SMAPI {version}（{}）", format_bytes(received)),
        };
        Self {
            phase: "download".to_string(),
            message,
            received,
            total,
            percent,
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<InstallerAsset>,
}

/// Pick the player installer zip, not source archives or the developer package.
pub fn select_installer_asset(assets: &[InstallerAsset]) -> AppResult<&InstallerAsset> {
    assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.ends_with("-installer.zip") && !name.contains("developer")
        })
        .ok_or_else(|| AppError::new("smapi_release_missing", "未找到 SMAPI 安装包"))
}

pub fn installer_file_name() -> &'static str {
    #[cfg(windows)]
    {
        "SMAPI.Installer.exe"
    }
    #[cfg(not(windows))]
    {
        "SMAPI.Installer"
    }
}

pub fn platform_folder() -> &'static str {
    #[cfg(windows)]
    {
        "windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "linux"
    }
}

/// Locate the platform installer inside an extracted release zip.
pub fn find_installer(root: &Path, platform_folder: &str, file_name: &str) -> AppResult<PathBuf> {
    let mut matches = Vec::new();
    collect_installers(root, platform_folder, file_name, &mut matches)?;
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(AppError::new(
            "smapi_installer_missing",
            "安装包里没有当前系统的 SMAPI 安装程序",
        )),
        _ => Err(AppError::new(
            "smapi_installer_ambiguous",
            "安装包里有多个安装程序，无法确定要运行哪一个",
        )),
    }
}

pub fn installer_args(game_path: &Path) -> Vec<String> {
    vec![
        "--install".to_string(),
        "--game-path".to_string(),
        game_path.display().to_string(),
        "--no-prompt".to_string(),
    ]
}

fn game_marker_present(game_path: &Path) -> bool {
    #[cfg(windows)]
    {
        game_path.join("Stardew Valley.exe").is_file()
    }
    #[cfg(not(windows))]
    {
        game_path.join("Stardew Valley").is_file()
            || game_path.join("StardewValley").is_file()
            || game_path.join("Stardew Valley.dll").is_file()
            || game_path.join("StardewValley.dll").is_file()
    }
}

pub fn ensure_game_dir(game_path: &Path) -> AppResult<()> {
    if !game_path.is_dir() {
        return Err(AppError::new("game_path_invalid", "游戏目录无效")
            .with_detail(game_path.display().to_string()));
    }
    if !game_marker_present(game_path) {
        return Err(AppError::new(
            "game_executable_missing",
            "该目录不像星露谷物语安装目录，请先在设置中指定游戏路径",
        )
        .with_detail(game_path.display().to_string()));
    }
    Ok(())
}

/// Download the latest official installer and run it against `game_path`.
pub fn install_into(game_path: &Path, report: &dyn Fn(InstallProgress)) -> AppResult<String> {
    report(InstallProgress::step("query", "正在获取 SMAPI 版本…"));
    ensure_game_dir(game_path)?;
    let release = fetch_latest_release()?;
    let asset = select_installer_asset(&release.assets)?.clone();
    let work = temp_work_dir()?;
    let result = install_from_asset(game_path, &release.tag_name, &asset, &work, report);
    let _ = fs::remove_dir_all(&work);
    result
}

fn install_from_asset(
    game_path: &Path,
    version: &str,
    asset: &InstallerAsset,
    work: &Path,
    report: &dyn Fn(InstallProgress),
) -> AppResult<String> {
    let zip_path = work.join("installer.zip");
    download_file(&asset.url, &zip_path, version, report)?;
    report(InstallProgress::step("extract", "正在解压安装包…"));
    let extracted = work.join("extracted");
    fs::create_dir_all(&extracted).map_err(|e| {
        AppError::new("smapi_extract_failed", "无法创建解压目录").with_detail(e.to_string())
    })?;
    extract_zip(&zip_path, &extracted)?;
    clear_quarantine(&extracted);
    let installer = find_installer(&extracted, platform_folder(), installer_file_name())?;
    make_executable(&installer)?;
    report(InstallProgress::step("install", "正在安装 SMAPI…"));
    run_installer(&installer, game_path)?;

    let smapi_path = game_path.join(smapi_file_name());
    if !smapi_path.is_file() {
        return Err(
            AppError::new("smapi_install_incomplete", "安装程序已结束，但未找到 SMAPI")
                .with_detail(smapi_path.display().to_string()),
        );
    }
    let mods = game_path.join("Mods");
    if !mods.is_dir() {
        fs::create_dir_all(&mods).map_err(|e| {
            AppError::new("mods_dir_create_failed", "无法创建 Mods 目录").with_detail(e.to_string())
        })?;
    }
    Ok(version.to_string())
}

fn fetch_latest_release() -> AppResult<GithubRelease> {
    let response = http_client()?
        .get(RELEASES_URL)
        .header("User-Agent", "SVMM")
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| {
            AppError::new("smapi_release_failed", "无法获取 SMAPI 版本信息")
                .with_detail(e.to_string())
        })?;
    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        return fetch_latest_release_from_page();
    }
    if !status.is_success() {
        return Err(
            AppError::new("smapi_release_failed", "无法获取 SMAPI 版本信息")
                .with_detail(status.to_string()),
        );
    }
    response.json().map_err(|e| {
        AppError::new("smapi_release_failed", "无法解析 SMAPI 版本信息").with_detail(e.to_string())
    })
}

/// `https://github.com/Pathoschild/SMAPI/releases/latest` redirects to `/releases/tag/<tag>`.
fn fetch_latest_release_from_page() -> AppResult<GithubRelease> {
    let response = http_client()?
        .get(RELEASES_PAGE)
        .header("User-Agent", "SVMM")
        .send()
        .map_err(|e| {
            AppError::new("smapi_release_failed", "无法获取 SMAPI 版本信息")
                .with_detail(e.to_string())
        })?;
    let status = response.status();
    if !status.is_success() {
        return Err(
            AppError::new("smapi_release_failed", "无法获取 SMAPI 版本信息")
                .with_detail(status.to_string()),
        );
    }
    let url = response.url().to_string();
    let tag = release_tag_from_url(&url)
        .map(str::to_string)
        .ok_or_else(|| {
            AppError::new("smapi_release_failed", "无法解析 SMAPI 版本信息").with_detail(url)
        })?;
    Ok(release_from_tag(&tag))
}

pub fn release_tag_from_url(url: &str) -> Option<&str> {
    let path = url.split(['?', '#']).next()?;
    let marker = "/releases/tag/";
    let start = path.find(marker)? + marker.len();
    let tag = path[start..].trim_matches('/');
    if tag.is_empty() || tag.contains('/') {
        None
    } else {
        Some(tag)
    }
}

fn release_from_tag(tag: &str) -> GithubRelease {
    let version = tag.trim_start_matches('v');
    let name = format!("SMAPI-{version}-installer.zip");
    let url = format!("https://github.com/Pathoschild/SMAPI/releases/download/{tag}/{name}");
    GithubRelease {
        tag_name: version.to_string(),
        assets: vec![InstallerAsset { name, url }],
    }
}

fn download_file(
    url: &str,
    dest: &Path,
    version: &str,
    report: &dyn Fn(InstallProgress),
) -> AppResult<()> {
    if !url.to_ascii_lowercase().starts_with("https://") {
        return Err(AppError::new(
            "smapi_download_unsafe",
            "安装包地址不是 HTTPS",
        ));
    }
    let mut response = http_client()?
        .get(url)
        .header("User-Agent", "SVMM")
        .send()
        .map_err(|e| {
            AppError::new("smapi_download_failed", "下载 SMAPI 安装包失败")
                .with_detail(e.to_string())
        })?;
    if let Some(final_url) = response.url().as_str().split('?').next() {
        if !final_url.to_ascii_lowercase().starts_with("https://") {
            return Err(AppError::new(
                "smapi_download_unsafe",
                "安装包重定向到了非 HTTPS 地址",
            ));
        }
    }
    let status = response.status();
    if !status.is_success() {
        return Err(
            AppError::new("smapi_download_failed", "下载 SMAPI 安装包失败")
                .with_detail(status.to_string()),
        );
    }
    let total = response.content_length();
    let mut file = File::create(dest).map_err(|e| {
        AppError::new("smapi_download_failed", "无法保存 SMAPI 安装包").with_detail(e.to_string())
    })?;
    let mut received = 0u64;
    let mut buf = [0u8; 64 * 1024];
    let mut last_report = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_secs(1))
        .unwrap_or_else(std::time::Instant::now);
    report(InstallProgress::download(version, 0, total));
    loop {
        let n = response.read(&mut buf).map_err(|e| {
            AppError::new("smapi_download_failed", "下载 SMAPI 安装包失败")
                .with_detail(e.to_string())
        })?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| {
            AppError::new("smapi_download_failed", "无法保存 SMAPI 安装包")
                .with_detail(e.to_string())
        })?;
        received += n as u64;
        if last_report.elapsed() >= std::time::Duration::from_millis(200)
            || total.is_some_and(|total| received >= total)
        {
            report(InstallProgress::download(version, received, total));
            last_report = std::time::Instant::now();
        }
    }
    report(InstallProgress::download(
        version,
        received,
        total.or(Some(received)),
    ));
    Ok(())
}

fn http_client() -> AppResult<reqwest::blocking::Client> {
    http::blocking_client(Duration::from_secs(REQUEST_TIMEOUT_SECS))
}

fn extract_zip(zip_path: &Path, dest: &Path) -> AppResult<()> {
    let file = File::open(zip_path).map_err(|e| {
        AppError::new("smapi_extract_failed", "无法打开 SMAPI 安装包").with_detail(e.to_string())
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        AppError::new("smapi_extract_failed", "SMAPI 安装包不是有效的 zip")
            .with_detail(e.to_string())
    })?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            AppError::new("smapi_extract_failed", "无法读取安装包条目").with_detail(e.to_string())
        })?;
        let name = match entry.enclosed_name() {
            Some(path) => path.to_string_lossy().replace('\\', "/"),
            None => {
                return Err(AppError::new("unsafe_zip", "安装包包含不安全路径")
                    .with_detail(entry.name().to_string()));
            }
        };
        let is_dir = entry.is_dir() || name.ends_with('/');
        if is_dir {
            let dir_name = name.trim_end_matches('/');
            if !dir_name.is_empty() {
                let dir = validate_zip_entry_path(dir_name, dest)?;
                fs::create_dir_all(&dir).map_err(|e| {
                    AppError::new("smapi_extract_failed", "无法创建目录").with_detail(e.to_string())
                })?;
            }
            continue;
        }
        let out_path = validate_zip_entry_path(&name, dest)?;
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AppError::new("smapi_extract_failed", "无法创建目录").with_detail(e.to_string())
            })?;
        }
        let mut outfile = File::create(&out_path).map_err(|e| {
            AppError::new("smapi_extract_failed", "无法写入安装文件").with_detail(e.to_string())
        })?;
        io::copy(&mut entry, &mut outfile).map_err(|e| {
            AppError::new("smapi_extract_failed", "无法写入安装文件").with_detail(e.to_string())
        })?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&out_path, fs::Permissions::from_mode(mode));
        }
    }
    Ok(())
}

fn run_installer(installer: &Path, game_path: &Path) -> AppResult<()> {
    let mut cmd = Command::new(installer);
    if let Some(dir) = installer.parent() {
        cmd.current_dir(dir);
    }
    cmd.args(installer_args(game_path));
    cmd.stdin(Stdio::null());
    let output = cmd.output().map_err(|e| {
        AppError::new("smapi_install_failed", "无法启动 SMAPI 安装程序").with_detail(e.to_string())
    })?;
    if output.status.success() {
        return Ok(());
    }
    let detail = output_tail(&output);
    Err(AppError::new("smapi_install_failed", "SMAPI 安装程序失败").with_detail(detail))
}

fn output_tail(output: &std::process::Output) -> String {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    let text = text.trim();
    if text.is_empty() {
        return format!("exit {}", output.status);
    }
    const MAX: usize = 800;
    let count = text.chars().count();
    if count <= MAX {
        text.to_string()
    } else {
        text.chars().skip(count - MAX).collect()
    }
}

fn collect_installers(
    dir: &Path,
    platform_folder: &str,
    file_name: &str,
    out: &mut Vec<PathBuf>,
) -> AppResult<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| {
        AppError::new("smapi_extract_failed", "无法读取安装包目录").with_detail(e.to_string())
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| {
            AppError::new("smapi_extract_failed", "无法读取安装包目录").with_detail(e.to_string())
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_installers(&path, platform_folder, file_name, out)?;
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.eq_ignore_ascii_case(file_name) {
            continue;
        }
        let in_platform = path.components().any(|component| {
            component
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(platform_folder)
        });
        if in_platform {
            out.push(path);
        }
    }
    Ok(())
}

fn temp_work_dir() -> AppResult<PathBuf> {
    let dir = std::env::temp_dir().join(format!(
        "svmm-smapi-install-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&dir).map_err(|e| {
        AppError::new("smapi_install_failed", "无法创建临时目录").with_detail(e.to_string())
    })?;
    Ok(dir)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .map_err(|e| {
            AppError::new("smapi_install_failed", "无法读取安装程序权限").with_detail(e.to_string())
        })?
        .permissions();
    perms.set_mode(perms.mode() | 0o755);
    fs::set_permissions(path, perms).map_err(|e| {
        AppError::new("smapi_install_failed", "无法标记安装程序为可执行").with_detail(e.to_string())
    })?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> AppResult<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn clear_quarantine(root: &Path) {
    let _ = Command::new("xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(root)
        .status();
}

#[cfg(not(target_os = "macos"))]
fn clear_quarantine(_root: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> InstallerAsset {
        InstallerAsset {
            name: name.to_string(),
            url: format!("https://example.invalid/{name}"),
        }
    }

    #[test]
    fn select_prefers_player_installer_zip() {
        let assets = vec![
            asset("SMAPI-4.5.2-installer-for-developers.zip"),
            asset("Source code.zip"),
            asset("SMAPI-4.5.2-installer.zip"),
        ];
        let picked = select_installer_asset(&assets).unwrap();
        assert_eq!(picked.name, "SMAPI-4.5.2-installer.zip");
    }

    #[test]
    fn select_errors_when_installer_zip_missing() {
        let err = select_installer_asset(&[asset("README.txt")]).unwrap_err();
        assert_eq!(err.code, "smapi_release_missing");
    }

    #[test]
    fn find_installer_uses_platform_folder() {
        let root = std::env::temp_dir().join(format!(
            "svmm-smapi-find-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let macos = root.join("bundle").join("internal").join("macOS");
        let linux = root.join("bundle").join("internal").join("linux");
        fs::create_dir_all(&macos).unwrap();
        fs::create_dir_all(&linux).unwrap();
        fs::write(macos.join("SMAPI.Installer"), b"").unwrap();
        fs::write(linux.join("SMAPI.Installer"), b"").unwrap();

        let found = find_installer(&root, "macos", "SMAPI.Installer").unwrap();
        assert_eq!(found, macos.join("SMAPI.Installer"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn release_tag_from_latest_redirect() {
        assert_eq!(
            release_tag_from_url("https://github.com/Pathoschild/SMAPI/releases/tag/4.5.2"),
            Some("4.5.2")
        );
        assert_eq!(
            release_tag_from_url(
                "https://github.com/Pathoschild/SMAPI/releases/tag/4.5.2?no-cache=1"
            ),
            Some("4.5.2")
        );
        assert_eq!(
            release_tag_from_url("https://github.com/Pathoschild/SMAPI"),
            None
        );
    }

    #[test]
    fn release_from_tag_builds_player_installer_url() {
        let release = release_from_tag("4.5.2");
        assert_eq!(release.tag_name, "4.5.2");
        let asset = select_installer_asset(&release.assets).unwrap();
        assert_eq!(asset.name, "SMAPI-4.5.2-installer.zip");
        assert_eq!(
            asset.url,
            "https://github.com/Pathoschild/SMAPI/releases/download/4.5.2/SMAPI-4.5.2-installer.zip"
        );
    }

    #[test]
    fn installer_args_are_noninteractive() {
        let args = installer_args(Path::new("/Games/Stardew Valley"));
        assert_eq!(
            args,
            vec![
                "--install",
                "--game-path",
                "/Games/Stardew Valley",
                "--no-prompt",
            ]
        );
    }
}
