use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::storage::secure_key;

pub const APPLICATION_NAME: &str = "SVMM";
pub const APPLICATION_VERSION: &str = "0.1.0";
pub const API_MODS_BASE: &str = "https://api.nexusmods.com/v1/games/stardewvalley/mods";
const VALIDATE_URL: &str = "https://api.nexusmods.com/v1/users/validate.json";
const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NexusUser {
    pub name: String,
    #[serde(alias = "is_premium")]
    pub is_premium: bool,
    #[serde(alias = "is_supporter")]
    pub is_supporter: bool,
}

/// Pure header map for Nexus API requests (no network).
pub fn build_nexus_headers(api_key: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert("apikey".to_string(), api_key.to_string());
    headers.insert("Application-Name".to_string(), APPLICATION_NAME.to_string());
    headers.insert(
        "Application-Version".to_string(),
        APPLICATION_VERSION.to_string(),
    );
    headers
}

/// Pure URL builder for endorse endpoint (no network).
pub fn endorse_mod_url(mod_id: u32) -> String {
    format!("{API_MODS_BASE}/{mod_id}/endorse.json")
}

/// Pure URL builder for mod files list (no network).
pub fn mod_files_url(mod_id: u32) -> String {
    format!("{API_MODS_BASE}/{mod_id}/files.json")
}

/// Pure URL builder for download_link endpoint (no network).
pub fn download_link_url(mod_id: u32, file_id: u64) -> String {
    format!("{API_MODS_BASE}/{mod_id}/files/{file_id}/download_link.json")
}

/// Parse `Nexus:1234` from update keys (case-insensitive `Nexus:` prefix).
pub fn parse_nexus_mod_id(update_keys: &[String]) -> Option<u32> {
    for key in update_keys {
        let trimmed = key.trim();
        let lower = trimmed.to_ascii_lowercase();
        let Some(rest) = lower.strip_prefix("nexus:") else {
            continue;
        };
        if let Ok(id) = rest.trim().parse::<u32>() {
            return Some(id);
        }
    }
    None
}

pub fn validate_nexus_api_key() -> AppResult<NexusUser> {
    let api_key = secure_key::get_nexus_api_key()?;
    let headers = build_nexus_headers(&api_key);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|_| AppError::new("nexus_client_failed", "无法创建 Nexus HTTP 客户端"))?;

    let mut request = client.get(VALIDATE_URL);
    for (name, value) in &headers {
        request = request.header(name.as_str(), value.as_str());
    }

    let response = request
        .send()
        .map_err(|_| AppError::new("nexus_validate_failed", "无法连接 Nexus Mods API"))?;

    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(AppError::new(
            "nexus_unauthorized",
            "Nexus API 密钥无效或已过期",
        ));
    }
    if !status.is_success() {
        return Err(AppError::new(
            "nexus_validate_failed",
            format!("Nexus 验证失败（HTTP {}）", status.as_u16()),
        ));
    }

    response
        .json::<NexusUser>()
        .map_err(|_| AppError::new("nexus_validate_parse_failed", "无法解析 Nexus 用户信息"))
}

#[derive(Serialize)]
struct EndorseBody {
    #[serde(rename = "Version")]
    version: String,
}

/// POST endorse for a Nexus mod. Never logs the API key.
pub fn endorse_mod(mod_id: u32, version: Option<&str>) -> AppResult<()> {
    let api_key = secure_key::get_nexus_api_key()?;
    let headers = build_nexus_headers(&api_key);
    let url = endorse_mod_url(mod_id);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|_| AppError::new("nexus_client_failed", "无法创建 Nexus HTTP 客户端"))?;

    let mut request = client.post(&url);
    for (name, value) in &headers {
        request = request.header(name.as_str(), value.as_str());
    }

    let response = match version.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => request.json(&EndorseBody {
            version: v.to_string(),
        }),
        None => request.json(&serde_json::json!({})),
    }
    .send()
    .map_err(|_| AppError::new("nexus_endorse_failed", "无法连接 Nexus Mods API"))?;

    let status = response.status();
    if status.as_u16() == 401 {
        return Err(AppError::new(
            "nexus_unauthorized",
            "Nexus API 密钥无效或已过期",
        ));
    }
    if status.as_u16() == 403 {
        return Err(AppError::new(
            "nexus_endorse_forbidden",
            "无权推荐该模组（可能尚未下载或已推荐）",
        ));
    }
    if !status.is_success() {
        return Err(AppError::new(
            "nexus_endorse_failed",
            format!("推荐失败（HTTP {}）", status.as_u16()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_nexus_headers_includes_apikey_and_app_meta() {
        let headers = build_nexus_headers("dummy-test-key");
        assert_eq!(
            headers.get("apikey").map(String::as_str),
            Some("dummy-test-key")
        );
        assert_eq!(
            headers.get("Application-Name").map(String::as_str),
            Some("SVMM")
        );
        assert_eq!(
            headers.get("Application-Version").map(String::as_str),
            Some("0.1.0")
        );
        assert_eq!(headers.len(), 3);
    }

    #[test]
    fn nexus_user_deserializes_snake_case_aliases() {
        let json = r#"{
            "name": "TestUser",
            "is_premium": true,
            "is_supporter": false
        }"#;
        let user: NexusUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.name, "TestUser");
        assert!(user.is_premium);
        assert!(!user.is_supporter);
    }

    #[test]
    fn endorse_mod_url_path() {
        assert_eq!(
            endorse_mod_url(1915),
            "https://api.nexusmods.com/v1/games/stardewvalley/mods/1915/endorse.json"
        );
    }

    #[test]
    fn download_link_url_path() {
        assert_eq!(
            download_link_url(1915, 42001),
            "https://api.nexusmods.com/v1/games/stardewvalley/mods/1915/files/42001/download_link.json"
        );
    }

    #[test]
    fn mod_files_url_path() {
        assert_eq!(
            mod_files_url(42),
            "https://api.nexusmods.com/v1/games/stardewvalley/mods/42/files.json"
        );
    }

    #[test]
    fn parse_nexus_mod_id_case_insensitive() {
        assert_eq!(
            parse_nexus_mod_id(&[String::from("Nexus:1915")]),
            Some(1915)
        );
        assert_eq!(
            parse_nexus_mod_id(&[String::from("nexus:99")]),
            Some(99)
        );
        assert_eq!(
            parse_nexus_mod_id(&[String::from("NEXUS:7")]),
            Some(7)
        );
        assert_eq!(
            parse_nexus_mod_id(&[String::from("Chucklefish:1"), String::from("Nexus:3")]),
            Some(3)
        );
        assert_eq!(parse_nexus_mod_id(&[String::from("GitHub:foo")]), None);
    }
}
