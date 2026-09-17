use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::mods::scan::ModEntry;
use crate::error::{AppError, AppResult};
use crate::storage::paths;

const SMAPI_UPDATE_URL: &str = "https://smapi.io/api/v3.0/mods";
const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub id: String,
    pub status: String,
    pub suggested_version: Option<String>,
    pub error_reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRequest {
    mods: Vec<UpdateRequestMod>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRequestMod {
    id: String,
    update_keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureModUpdate {
    suggested_update: Option<SuggestedUpdate>,
    #[serde(default)]
    errors: Vec<String>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SuggestedUpdate {
    version: Option<String>,
}

/// Map a simplified SMAPI-style fixture object (keyed by mod id) to `UpdateInfo` list.
pub fn parse_update_fixture(map: &HashMap<String, Value>) -> Vec<UpdateInfo> {
    let mut out = Vec::with_capacity(map.len());
    for (id, value) in map {
        let entry: FixtureModUpdate = match serde_json::from_value(value.clone()) {
            Ok(e) => e,
            Err(_) => {
                out.push(UpdateInfo {
                    id: id.clone(),
                    status: "ok".to_string(),
                    suggested_version: None,
                    error_reason: None,
                });
                continue;
            }
        };
        out.push(map_fixture_entry(id, &entry));
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn map_fixture_entry(id: &str, entry: &FixtureModUpdate) -> UpdateInfo {
    let suggested_version = entry
        .suggested_update
        .as_ref()
        .and_then(|s| s.version.clone())
        .filter(|v| !v.is_empty());

    let unofficial = text_contains_unofficial(&entry.errors, &entry.metadata);
    let error_reason = if entry.errors.is_empty() {
        None
    } else {
        Some(entry.errors.join("; "))
    };

    let status = if !entry.errors.is_empty() {
        if unofficial {
            "unofficial_update".to_string()
        } else {
            "broken".to_string()
        }
    } else if suggested_version.is_some() {
        if unofficial {
            "unofficial_update".to_string()
        } else {
            "update_available".to_string()
        }
    } else {
        "ok".to_string()
    };

    UpdateInfo {
        id: id.to_string(),
        status,
        suggested_version,
        error_reason,
    }
}

fn text_contains_unofficial(errors: &[String], metadata: &Value) -> bool {
    if errors.iter().any(|e| e.to_ascii_lowercase().contains("unofficial")) {
        return true;
    }
    metadata
        .to_string()
        .to_ascii_lowercase()
        .contains("unofficial")
}

/// Merge update status into a mod entry without clobbering scan-time issues,
/// unless the update result is `broken`.
pub fn merge_update_status(current_status: &str, update_status: &str) -> String {
    if (current_status == "missing_manifest" || current_status == "incompatible")
        && update_status != "broken"
    {
        return current_status.to_string();
    }
    update_status.to_string()
}

/// Check SMAPI for updates for mods that have `update_keys`.
pub fn check_updates(mods: &[ModEntry]) -> AppResult<Vec<UpdateInfo>> {
    let request_mods: Vec<UpdateRequestMod> = mods
        .iter()
        .filter(|m| !m.id.is_empty() && !m.update_keys.is_empty())
        .map(|m| UpdateRequestMod {
            id: m.id.clone(),
            update_keys: m.update_keys.clone(),
        })
        .collect();

    if request_mods.is_empty() {
        return Ok(Vec::new());
    }

    let body = UpdateRequest { mods: request_mods };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::new("update_check_failed", "无法创建 HTTP 客户端").with_detail(e.to_string()))?;

    let response = client
        .post(SMAPI_UPDATE_URL)
        .json(&body)
        .send()
        .map_err(|e| {
            AppError::new("update_check_failed", "检查更新失败（网络错误）").with_detail(e.to_string())
        })?;

    if !response.status().is_success() {
        return Err(AppError::new(
            "update_check_failed",
            format!("检查更新失败（HTTP {}）", response.status()),
        ));
    }

    let value: Value = response.json().map_err(|e| {
        AppError::new("update_check_failed", "无法解析更新响应").with_detail(e.to_string())
    })?;

    let map = match value {
        Value::Object(obj) => obj.into_iter().collect::<HashMap<_, _>>(),
        _ => {
            return Err(AppError::new(
                "update_check_failed",
                "更新响应格式无效（期望对象）",
            ));
        }
    };

    let infos = parse_update_fixture(&map);
    let _ = write_update_cache(&infos);
    Ok(infos)
}

fn write_update_cache(infos: &[UpdateInfo]) -> AppResult<()> {
    let dir = paths::app_data_dir().join("cache");
    fs::create_dir_all(&dir).map_err(|e| {
        AppError::new("cache_write_failed", "无法创建更新缓存目录").with_detail(e.to_string())
    })?;
    let path = dir.join("update_check.json");
    let json = serde_json::to_string_pretty(infos).map_err(|e| {
        AppError::new("cache_write_failed", "无法序列化更新缓存").with_detail(e.to_string())
    })?;
    fs::write(&path, json).map_err(|e| {
        AppError::new("cache_write_failed", "无法写入更新缓存").with_detail(e.to_string())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_update_fixture_maps_statuses() {
        let raw = json!({
            "Ada.TestMod": {
                "suggestedUpdate": { "version": "2.0.0" },
                "errors": [],
                "metadata": { "main": { "status": "optional" } }
            },
            "Broken.Mod": {
                "suggestedUpdate": null,
                "errors": ["incompatible"],
                "metadata": {}
            }
        });
        let map: HashMap<String, Value> = serde_json::from_value(raw).unwrap();
        let infos = parse_update_fixture(&map);
        assert_eq!(infos.len(), 2);

        let ada = infos.iter().find(|i| i.id == "Ada.TestMod").unwrap();
        assert_eq!(ada.status, "update_available");
        assert_eq!(ada.suggested_version.as_deref(), Some("2.0.0"));
        assert!(ada.error_reason.is_none());

        let broken = infos.iter().find(|i| i.id == "Broken.Mod").unwrap();
        assert_eq!(broken.status, "broken");
        assert_eq!(broken.error_reason.as_deref(), Some("incompatible"));
        assert!(broken.suggested_version.is_none());
    }

    #[test]
    fn merge_preserves_scan_status_unless_broken() {
        assert_eq!(
            merge_update_status("missing_manifest", "update_available"),
            "missing_manifest"
        );
        assert_eq!(
            merge_update_status("incompatible", "ok"),
            "incompatible"
        );
        assert_eq!(merge_update_status("missing_manifest", "broken"), "broken");
        assert_eq!(merge_update_status("ok", "update_available"), "update_available");
    }
}
