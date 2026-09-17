use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::domain::mods::install::install_mod_zip;
use crate::domain::mods::scan::ModEntry;
use crate::domain::nexus::client::build_nexus_headers;
use crate::error::{AppError, AppResult};
use crate::storage::secure_key;

const REQUEST_TIMEOUT_SECS: u64 = 60;
const DOWNLOAD_LINK_BASE: &str =
    "https://api.nexusmods.com/v1/games/stardewvalley/mods";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NxmUrl {
    pub game: String,
    pub mod_id: u64,
    pub file_id: u64,
    pub key: Option<String>,
    pub expires: Option<String>,
    /// Other query pairs (excluding key/expires) forwarded to the download_link API.
    pub extra_query: Vec<(String, String)>,
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

/// Fetch the CDN download URL for a Nexus file (uses stored API key + NXM query params).
pub fn fetch_download_link(nxm: &NxmUrl) -> AppResult<String> {
    let api_key = secure_key::get_nexus_api_key()?;
    let headers = build_nexus_headers(&api_key);

    let mut url = format!(
        "{DOWNLOAD_LINK_BASE}/{}/files/{}/download_link.json",
        nxm.mod_id, nxm.file_id
    );
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

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|_| AppError::new("nexus_client_failed", "无法创建 Nexus HTTP 客户端"))?;

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

pub fn download_url_to_temp_zip(cdn_url: &str) -> AppResult<PathBuf> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|_| AppError::new("nexus_client_failed", "无法创建下载客户端"))?;

    let response = client
        .get(cdn_url)
        .send()
        .map_err(|_| AppError::new("nexus_download_failed", "下载模组文件失败"))?;

    if !response.status().is_success() {
        return Err(AppError::new(
            "nexus_download_failed",
            format!("下载失败（HTTP {}）", response.status().as_u16()),
        ));
    }

    let bytes = response
        .bytes()
        .map_err(|_| AppError::new("nexus_download_failed", "读取下载内容失败"))?;

    let path = std::env::temp_dir().join(format!("svmm-nxm-{}.zip", uuid::Uuid::new_v4()));
    let mut file = File::create(&path).map_err(|e| {
        AppError::new("nexus_download_failed", "无法写入临时文件").with_detail(e.to_string())
    })?;
    file.write_all(&bytes).map_err(|e| {
        AppError::new("nexus_download_failed", "无法写入临时文件").with_detail(e.to_string())
    })?;
    Ok(path)
}

pub fn handle_nxm_url(url: &str, mods_path: &Path) -> AppResult<ModEntry> {
    let nxm = parse_nxm_url(url)?;
    let cdn = fetch_download_link(&nxm)?;
    let zip_path = download_url_to_temp_zip(&cdn)?;
    let result = install_mod_zip(&zip_path, mods_path);
    let _ = std::fs::remove_file(&zip_path);
    result
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
        // Error must not contain secrets (none present), just ensure code is correct.
    }

    #[test]
    fn parse_nxm_case_insensitive_game() {
        let nxm = parse_nxm_url("nxm://StardewValley/mods/10/files/20").unwrap();
        assert_eq!(nxm.mod_id, 10);
        assert_eq!(nxm.file_id, 20);
    }

    #[test]
    fn parse_nxm_invalid_shape() {
        let err = parse_nxm_url("nxm://stardewvalley/mods/1").unwrap_err();
        assert_eq!(err.code, "nxm_invalid");
    }
}
