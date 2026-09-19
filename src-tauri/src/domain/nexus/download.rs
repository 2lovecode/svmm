use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::domain::mods::enable;
use crate::domain::mods::install::install_mod_zip;
use crate::domain::mods::scan::ModEntry;
use crate::domain::nexus::client::{
    build_nexus_headers, download_link_url, mod_files_url, parse_nexus_mod_id,
};
use crate::error::{AppError, AppResult};
use crate::storage::secure_key;

const REQUEST_TIMEOUT_SECS: u64 = 60;

#[derive(Clone, PartialEq, Eq)]
pub struct NxmUrl {
    pub game: String,
    pub mod_id: u64,
    pub file_id: u64,
    pub key: Option<String>,
    pub expires: Option<String>,
    /// Other query pairs (excluding key/expires) forwarded to the download_link API.
    pub extra_query: Vec<(String, String)>,
}

impl std::fmt::Debug for NxmUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NxmUrl")
            .field("game", &self.game)
            .field("mod_id", &self.mod_id)
            .field("file_id", &self.file_id)
            .field("key", &self.key.as_ref().map(|_| "[REDACTED]"))
            .field("expires", &self.expires)
            .field("extra_query", &self.extra_query)
            .finish()
    }
}

/// Parse `nxm://stardewvalley/mods/{modId}/files/{fileId}?key=&expires=&...`
/// Never logs key material.
pub fn parse_nxm_url(url: &str) -> AppResult<NxmUrl> {
    let trimmed = url.trim();
    let rest = trimmed
        .strip_prefix("nxm://")
        .or_else(|| trimmed.strip_prefix("NXM://"))
        .ok_or_else(|| AppError::new("nxm_invalid", "无效的 NXM 链接"))?;

    let (path_part, query_part) = match rest.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (rest, None),
    };

    let segments: Vec<&str> = path_part.split('/').filter(|s| !s.is_empty()).collect();
    // Expected: {game}/mods/{modId}/files/{fileId}
    if segments.len() != 5 || segments[1] != "mods" || segments[3] != "files" {
        return Err(AppError::new(
            "nxm_invalid",
            "NXM 链接格式不正确（需要 mods/{id}/files/{id}）",
        ));
    }

    let game = segments[0].to_string();
    if !game.eq_ignore_ascii_case("stardewvalley") {
        return Err(AppError::new(
            "nxm_unsupported_game",
            format!("仅支持星露谷物语（stardewvalley），当前为：{game}"),
        ));
    }

    let mod_id: u64 = segments[2]
        .parse()
        .map_err(|_| AppError::new("nxm_invalid", "无效的模组 ID"))?;
    let file_id: u64 = segments[4]
        .parse()
        .map_err(|_| AppError::new("nxm_invalid", "无效的文件 ID"))?;

    let mut key = None;
    let mut expires = None;
    let mut extra_query = Vec::new();
    if let Some(q) = query_part {
        for pair in q.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = match pair.split_once('=') {
                Some((k, v)) => (k, v),
                None => (pair, ""),
            };
            let k_decoded = urlencoding_decode(k);
            let v_decoded = urlencoding_decode(v);
            match k_decoded.as_str() {
                "key" => key = Some(v_decoded),
                "expires" => expires = Some(v_decoded),
                _ => extra_query.push((k_decoded, v_decoded)),
            }
        }
    }

    Ok(NxmUrl {
        game: "stardewvalley".to_string(),
        mod_id,
        file_id,
        key,
        expires,
        extra_query,
    })
}

fn urlencoding_decode(s: &str) -> String {
    // Minimal percent-decoding for Nexus query values.
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct DownloadLinkItem {
    #[serde(rename = "URI")]
    uri: String,
}

/// Page a non-premium user must open so Nexus can send an `nxm://` link back.
pub fn manual_download_page(mod_id: u32, file_id: u64) -> String {
    format!(
        "https://www.nexusmods.com/stardewvalley/mods/{mod_id}?tab=files&file_id={file_id}&nmm=1"
    )
}

fn premium_required_error() -> AppError {
    AppError::new(
        "nexus_premium_required",
        "需要 Nexus Premium 才能在应用内直接下载",
    )
}

fn looks_like_premium_required(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("premium")
        || lower.contains("only available for premium")
        || lower.contains("requires a premium")
}

/// Fetch the CDN download URL for a Nexus file (uses stored API key + NXM query params).
pub fn fetch_download_link(nxm: &NxmUrl) -> AppResult<String> {
    let api_key = secure_key::get_nexus_api_key()?;
    let headers = build_nexus_headers(&api_key);

    let mut url = download_link_url(nxm.mod_id as u32, nxm.file_id);
    let mut params: Vec<(String, String)> = Vec::new();
    if let Some(ref k) = nxm.key {
        params.push(("key".into(), k.clone()));
    }
    if let Some(ref e) = nxm.expires {
        params.push(("expires".into(), e.clone()));
    }
    for (k, v) in &nxm.extra_query {
        params.push((k.clone(), v.clone()));
    }
    if !params.is_empty() {
        let qs: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", encode_query(k), encode_query(v)))
            .collect();
        url.push('?');
        url.push_str(&qs.join("&"));
    }

    let client = crate::domain::http::blocking_client(Duration::from_secs(REQUEST_TIMEOUT_SECS))?;

    let mut request = client.get(&url);
    for (name, value) in &headers {
        request = request.header(name.as_str(), value.as_str());
    }

    let response = request
        .send()
        .map_err(|_| AppError::new("nexus_download_failed", "无法获取 Nexus 下载链接"))?;

    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(AppError::new(
            "nexus_unauthorized",
            "Nexus API 密钥无效或无权下载该文件",
        ));
    }
    if !status.is_success() {
        return Err(AppError::new(
            "nexus_download_failed",
            format!("获取下载链接失败（HTTP {}）", status.as_u16()),
        ));
    }

    let links: Vec<DownloadLinkItem> = response
        .json()
        .map_err(|_| AppError::new("nexus_download_parse_failed", "无法解析下载链接响应"))?;
    links
        .into_iter()
        .next()
        .map(|l| l.uri)
        .ok_or_else(|| AppError::new("nexus_download_empty", "Nexus 未返回可用下载地址"))
}

/// Premium users get a CDN link. Everyone else gets the Nexus file page that calls back via `nxm://`.
pub enum NexusAcquire {
    Direct(String),
    ManualPage(String),
}

pub fn acquire_nexus_file(mod_id: u32, file_id: u64) -> AppResult<NexusAcquire> {
    match fetch_premium_download_link(mod_id, file_id) {
        Ok(url) => Ok(NexusAcquire::Direct(url)),
        Err(err) if err.code == "nexus_premium_required" => {
            Ok(NexusAcquire::ManualPage(manual_download_page(mod_id, file_id)))
        }
        Err(err) => Err(err),
    }
}

/// Premium quick-download link (no NXM key/expires). Maps 403 → nexus_premium_required.
pub fn fetch_premium_download_link(mod_id: u32, file_id: u64) -> AppResult<String> {
    let api_key = secure_key::get_nexus_api_key()?;
    let headers = build_nexus_headers(&api_key);
    let url = download_link_url(mod_id, file_id);

    let client = crate::domain::http::blocking_client(Duration::from_secs(REQUEST_TIMEOUT_SECS))?;

    let mut request = client.get(&url);
    for (name, value) in &headers {
        request = request.header(name.as_str(), value.as_str());
    }

    let response = request
        .send()
        .map_err(|_| AppError::new("nexus_download_failed", "无法获取 Nexus 下载链接"))?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|_| AppError::new("nexus_download_failed", "无法读取下载链接响应"))?;

    if status.as_u16() == 403 || looks_like_premium_required(&body) {
        return Err(premium_required_error());
    }
    if status.as_u16() == 401 {
        return Err(AppError::new(
            "nexus_unauthorized",
            "Nexus API 密钥无效或已过期",
        ));
    }
    if !status.is_success() {
        return Err(AppError::new(
            "nexus_download_failed",
            format!("获取下载链接失败（HTTP {}）", status.as_u16()),
        ));
    }

    let links: Vec<DownloadLinkItem> = serde_json::from_str(&body)
        .map_err(|_| AppError::new("nexus_download_parse_failed", "无法解析下载链接响应"))?;
    links
        .into_iter()
        .next()
        .map(|l| l.uri)
        .ok_or_else(|| AppError::new("nexus_download_empty", "Nexus 未返回可用下载地址"))
}

fn encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Soft cap for a single Nexus zip download (512 MiB).
pub const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;

fn assert_https_download_url(cdn_url: &str) -> AppResult<()> {
    let trimmed = cdn_url.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("https://") {
        return Err(AppError::new(
            "nexus_download_unsafe_url",
            "仅允许通过 HTTPS 下载模组文件",
        ));
    }
    Ok(())
}

/// Stream a CDN zip to a temp file (never buffers the whole archive in RAM).
pub fn download_url_to_temp_zip(cdn_url: &str) -> AppResult<PathBuf> {
    use std::io::Read;

    assert_https_download_url(cdn_url)?;

    let client = crate::domain::http::blocking_client(Duration::from_secs(REQUEST_TIMEOUT_SECS))?;

    let mut response = client
        .get(cdn_url)
        .send()
        .map_err(|_| AppError::new("nexus_download_failed", "下载模组文件失败"))?;

    // After redirects, confirm final URL is still HTTPS (do not leak query in errors).
    if let Some(final_url) = response.url().as_str().split('?').next() {
        if !final_url.to_ascii_lowercase().starts_with("https://") {
            return Err(AppError::new(
                "nexus_download_unsafe_url",
                "下载重定向到了非 HTTPS 地址",
            ));
        }
    }

    if !response.status().is_success() {
        return Err(AppError::new(
            "nexus_download_failed",
            format!("下载失败（HTTP {}）", response.status().as_u16()),
        ));
    }

    if let Some(len) = response.content_length() {
        if len > MAX_DOWNLOAD_BYTES {
            return Err(AppError::new(
                "nexus_download_too_large",
                "模组文件过大，已取消下载",
            ));
        }
    }

    let path = std::env::temp_dir().join(format!("svmm-nxm-{}.zip", uuid::Uuid::new_v4()));
    let mut file = File::create(&path).map_err(|e| {
        AppError::new("nexus_download_failed", "无法写入临时文件").with_detail(e.to_string())
    })?;

    let mut buf = [0u8; 64 * 1024];
    let mut written: u64 = 0;
    loop {
        let n = response.read(&mut buf).map_err(|_| {
            let _ = fs::remove_file(&path);
            AppError::new("nexus_download_failed", "读取下载内容失败")
        })?;
        if n == 0 {
            break;
        }
        written = written.saturating_add(n as u64);
        if written > MAX_DOWNLOAD_BYTES {
            let _ = fs::remove_file(&path);
            return Err(AppError::new(
                "nexus_download_too_large",
                "模组文件过大，已取消下载",
            ));
        }
        file.write_all(&buf[..n]).map_err(|e| {
            let _ = fs::remove_file(&path);
            AppError::new("nexus_download_failed", "无法写入临时文件").with_detail(e.to_string())
        })?;
    }

    Ok(path)
}

pub fn handle_nxm_url(
    url: &str,
    library_root: &Path,
) -> AppResult<crate::domain::library::LibraryMod> {
    let nxm = parse_nxm_url(url)?;
    let cdn = fetch_download_link(&nxm)?;
    let zip_path = download_url_to_temp_zip(&cdn)?;
    let mods_path = crate::storage::settings::load_settings()
        .ok()
        .and_then(|settings| crate::domain::game::resolve_paths(&settings).ok())
        .map(|paths| paths.mods_path);
    let result = crate::domain::library::replace_from_zip(
        library_root,
        &crate::storage::paths::userdata_dir(),
        mods_path.as_deref(),
        &zip_path,
        crate::domain::library::ImportMeta {
            nexus_mod_id: u32::try_from(nxm.mod_id).ok(),
            nexus_file_id: Some(nxm.file_id),
            category: None,
        },
    );
    let _ = std::fs::remove_file(&zip_path);
    result?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::new("zip_no_mod", "压缩包中未找到有效的模组目录"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexusFileInfo {
    pub file_id: u64,
    pub name: String,
    pub version: String,
    pub category_name: String,
    pub uploaded_timestamp: i64,
    pub is_main: bool,
}

#[derive(Debug, Deserialize)]
struct FilesResponse {
    files: Vec<NexusFileMeta>,
}

#[derive(Debug, Deserialize)]
struct NexusFileMeta {
    file_id: u64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    category_id: Option<i64>,
    #[serde(default)]
    category_name: Option<String>,
    #[serde(default)]
    uploaded_timestamp: Option<i64>,
}

fn is_main_file(f: &NexusFileMeta) -> bool {
    f.category_id == Some(1)
        || f.category_name
            .as_deref()
            .is_some_and(|name| name.eq_ignore_ascii_case("MAIN"))
}

fn parse_files_response(files_json: &str) -> AppResult<FilesResponse> {
    serde_json::from_str(files_json).map_err(|err| {
        AppError::new("nexus_files_parse_failed", "无法解析 Nexus 文件列表")
            .with_detail(err.to_string())
    })
}

/// Pick newest MAIN file from a Nexus files.json payload (pure helper for tests).
#[cfg(test)]
pub fn pick_newest_main_file_id(files_json: &str) -> AppResult<u64> {
    let parsed = parse_files_response(files_json)?;
    parsed
        .files
        .into_iter()
        .filter(is_main_file)
        .max_by_key(|f| f.uploaded_timestamp.unwrap_or(0))
        .map(|f| f.file_id)
        .ok_or_else(|| AppError::new("nexus_no_main_file", "未找到可用的主文件"))
}

fn fetch_files_body(mod_id: u32) -> AppResult<String> {
    let api_key = secure_key::get_nexus_api_key()?;
    let headers = build_nexus_headers(&api_key);
    let url = mod_files_url(mod_id);
    let client = crate::domain::http::blocking_client(Duration::from_secs(REQUEST_TIMEOUT_SECS))?;
    let mut request = client.get(&url);
    for (name, value) in &headers {
        request = request.header(name.as_str(), value.as_str());
    }
    let response = request
        .send()
        .map_err(|_| AppError::new("nexus_files_failed", "无法获取 Nexus 文件列表"))?;
    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(AppError::new(
            "nexus_unauthorized",
            "Nexus API 密钥无效或无权访问该模组",
        ));
    }
    if !status.is_success() {
        return Err(AppError::new(
            "nexus_files_failed",
            format!("获取文件列表失败（HTTP {}）", status.as_u16()),
        ));
    }
    response
        .text()
        .map_err(|_| AppError::new("nexus_files_failed", "无法读取文件列表响应"))
}

pub fn parse_file_list(files_json: &str) -> AppResult<Vec<NexusFileInfo>> {
    let parsed = parse_files_response(files_json)?;
    let mut files: Vec<NexusFileInfo> = parsed
        .files
        .into_iter()
        .map(|f| NexusFileInfo {
            is_main: is_main_file(&f),
            file_id: f.file_id,
            name: f.name.unwrap_or_default(),
            version: f.version.unwrap_or_default(),
            category_name: f.category_name.unwrap_or_default(),
            uploaded_timestamp: f.uploaded_timestamp.unwrap_or(0),
        })
        .collect();
    files.sort_by(|a, b| b.uploaded_timestamp.cmp(&a.uploaded_timestamp));
    Ok(files)
}

pub fn list_nexus_files(mod_id: u32) -> AppResult<Vec<NexusFileInfo>> {
    parse_file_list(&fetch_files_body(mod_id)?)
}

fn fetch_newest_main_file_id(mod_id: u32) -> AppResult<u64> {
    let files = list_nexus_files(mod_id)?;
    files
        .into_iter()
        .filter(|f| f.is_main)
        .max_by_key(|f| f.uploaded_timestamp)
        .map(|f| f.file_id)
        .ok_or_else(|| AppError::new("nexus_no_main_file", "未找到可用的主文件"))
}

fn sanitize_backup_id(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | ' ' => '_',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_start_matches('.');
    if trimmed.is_empty() {
        "mod".to_string()
    } else {
        trimmed.chars().take(64).collect()
    }
}

fn resolve_mod_dir(mods_path: &Path, relative: &str) -> PathBuf {
    let mut path = mods_path.to_path_buf();
    for part in relative.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            continue;
        }
        path.push(part);
    }
    path
}

fn restore_backup(backup: &Path, original: &Path) -> AppResult<()> {
    if original.exists() {
        let _ = fs::remove_dir_all(original);
    }
    fs::rename(backup, original).map_err(|e| {
        AppError::new("nexus_update_restore_failed", "更新失败且无法恢复备份")
            .with_detail(e.to_string())
    })
}

/// Prefer restore failure when both update and restore fail (user must know backup state).
pub fn merge_update_and_restore_errors(update_err: AppError, restore_err: AppError) -> AppError {
    let update_part = match &update_err.detail {
        Some(d) if !d.is_empty() => format!("{}（{}）", update_err.message, d),
        _ => update_err.message.clone(),
    };
    let restore_part = match &restore_err.detail {
        Some(d) if !d.is_empty() => format!("{}（{}）", restore_err.message, d),
        _ => restore_err.message.clone(),
    };
    AppError::new("nexus_update_restore_failed", "更新失败且无法恢复备份")
        .with_detail(format!("更新错误：{update_part}；恢复错误：{restore_part}"))
}

/// After install, re-apply previous enabled/disabled folder naming.
pub fn preserve_enabled_state(
    mods_path: &Path,
    entry: ModEntry,
    was_enabled: bool,
) -> AppResult<ModEntry> {
    if was_enabled == entry.enabled {
        return Ok(entry);
    }
    enable::set_mod_enabled(mods_path, &entry.folder_path, was_enabled)
}

/// Download latest Nexus main file and replace the local mod folder (with backup).
pub fn update_mod_from_nexus(mod_entry: &ModEntry, mods_path: &Path) -> AppResult<ModEntry> {
    let nexus_id = parse_nexus_mod_id(&mod_entry.update_keys)
        .ok_or_else(|| AppError::new("nexus_id_missing", "该模组没有 Nexus 更新键（Nexus:ID）"))?;

    let file_id = fetch_newest_main_file_id(nexus_id)?;
    let cdn = fetch_premium_download_link(nexus_id, file_id)?;
    let zip_path = download_url_to_temp_zip(&cdn)?;

    let original = resolve_mod_dir(mods_path, &mod_entry.folder_path);
    if !original.is_dir() {
        let _ = fs::remove_file(&zip_path);
        return Err(AppError::new("mod_not_found", "未找到指定 mod 目录")
            .with_detail(original.display().to_string()));
    }

    let was_enabled = mod_entry.enabled;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_name = format!(".svmm-backup-{}-{}", sanitize_backup_id(&mod_entry.id), ts);
    let backup_path = mods_path.join(&backup_name);

    if let Err(e) = fs::rename(&original, &backup_path) {
        let _ = fs::remove_file(&zip_path);
        return Err(
            AppError::new("nexus_update_backup_failed", "无法备份现有模组目录")
                .with_detail(e.to_string()),
        );
    }

    let install_result = install_mod_zip(&zip_path, mods_path);
    let _ = fs::remove_file(&zip_path);

    match install_result {
        Ok(entry) => {
            let _ = fs::remove_dir_all(&backup_path);
            preserve_enabled_state(mods_path, entry, was_enabled)
        }
        Err(err) => match restore_backup(&backup_path, &original) {
            Ok(()) => Err(err),
            Err(restore_err) => Err(merge_update_and_restore_errors(err, restore_err)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_nxm_stardewvalley_happy_path() {
        let nxm = parse_nxm_url(
            "nxm://stardewvalley/mods/1915/files/12345?key=abc%2Bdef&expires=1700000000&user_id=9",
        )
        .unwrap();
        assert_eq!(nxm.game, "stardewvalley");
        assert_eq!(nxm.mod_id, 1915);
        assert_eq!(nxm.file_id, 12345);
        assert_eq!(nxm.key.as_deref(), Some("abc+def"));
        assert_eq!(nxm.expires.as_deref(), Some("1700000000"));
        assert_eq!(nxm.extra_query, vec![("user_id".into(), "9".into())]);
    }

    #[test]
    fn parse_nxm_rejects_other_game() {
        let err = parse_nxm_url("nxm://skyrim/mods/1/files/2").unwrap_err();
        assert_eq!(err.code, "nxm_unsupported_game");
    }

    #[test]
    fn parse_nxm_case_insensitive_game() {
        let nxm = parse_nxm_url("nxm://StardewValley/mods/10/files/20").unwrap();
        assert_eq!(nxm.mod_id, 10);
        assert_eq!(nxm.file_id, 20);
    }

    #[test]
    fn manual_download_page_asks_nexus_to_call_back() {
        assert_eq!(
            manual_download_page(2400, 99),
            "https://www.nexusmods.com/stardewvalley/mods/2400?tab=files&file_id=99&nmm=1"
        );
    }

    #[test]
    fn parse_nxm_invalid_shape() {
        let err = parse_nxm_url("nxm://stardewvalley/mods/1").unwrap_err();
        assert_eq!(err.code, "nxm_invalid");
    }

    #[test]
    fn nxm_url_debug_redacts_key() {
        let nxm = NxmUrl {
            game: "stardewvalley".into(),
            mod_id: 1,
            file_id: 2,
            key: Some("super-secret".into()),
            expires: Some("99".into()),
            extra_query: vec![],
        };
        let dbg = format!("{nxm:?}");
        assert!(dbg.contains("[REDACTED]"));
        assert!(!dbg.contains("super-secret"));
    }

    #[test]
    fn pick_newest_main_file_id_from_list() {
        let json = r#"{
          "files": [
            {"file_id": 10, "category_id": 1, "category_name": "MAIN", "uploaded_timestamp": 100},
            {"file_id": 20, "category_id": 1, "category_name": "MAIN", "uploaded_timestamp": 200},
            {"file_id": 30, "category_id": 3, "category_name": "OPTIONAL", "uploaded_timestamp": 999}
          ]
        }"#;
        assert_eq!(pick_newest_main_file_id(json).unwrap(), 20);
    }

    #[test]
    fn parse_file_list_accepts_null_category_name() {
        let json = r#"{
          "files": [
            {
              "id": [1, 2],
              "file_id": 10,
              "name": "Old",
              "version": "1.0",
              "category_id": 1,
              "category_name": null,
              "changelog_html": null,
              "uploaded_timestamp": 100
            },
            {
              "file_id": 20,
              "category_id": null,
              "category_name": "MAIN",
              "uploaded_timestamp": 200
            }
          ]
        }"#;
        let files = parse_file_list(json).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.is_main));
        assert_eq!(pick_newest_main_file_id(json).unwrap(), 20);
    }

    #[test]
    fn merge_update_and_restore_errors_propagates_both() {
        let update = AppError::new("zip_invalid", "无效的 zip 压缩包");
        let restore = AppError::new("nexus_update_restore_failed", "更新失败且无法恢复备份")
            .with_detail("access denied");
        let merged = merge_update_and_restore_errors(update, restore);
        assert_eq!(merged.code, "nexus_update_restore_failed");
        let detail = merged.detail.unwrap_or_default();
        assert!(detail.contains("无效的 zip"));
        assert!(detail.contains("access denied"));
    }

    #[test]
    fn preserve_enabled_state_disables_installed_folder() {
        let mods = std::env::temp_dir().join(format!(
            "svmm-preserve-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&mods);
        let mod_dir = mods.join("Fresh.Mod");
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(
            mod_dir.join("manifest.json"),
            br#"{
  "Name": "Fresh",
  "Author": "Ada",
  "Version": "2.0.0",
  "Description": "x",
  "UniqueID": "Fresh.Mod"
}"#,
        )
        .unwrap();

        let entry = crate::domain::mods::scan::entry_from_mod_dir(&mods, &mod_dir).unwrap();
        assert!(entry.enabled);
        let disabled = preserve_enabled_state(&mods, entry, false).unwrap();
        assert!(!disabled.enabled);
        assert!(mods.join(".Fresh.Mod").is_dir());
        let _ = fs::remove_dir_all(&mods);
    }

    #[test]
    fn download_rejects_non_https_url() {
        let err = download_url_to_temp_zip("http://example.com/mod.zip").unwrap_err();
        assert_eq!(err.code, "nexus_download_unsafe_url");
    }
}
