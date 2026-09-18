use serde::Deserialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Manifest {
    #[serde(rename = "UniqueID")]
    pub unique_id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub minimum_api_version: Option<String>,
    #[serde(default)]
    pub update_keys: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<ManifestDependency>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct ManifestDependency {
    #[serde(rename = "UniqueID")]
    pub unique_id: String,
    #[serde(default = "default_true")]
    pub is_required: bool,
}

fn default_true() -> bool {
    true
}

/// Strip a leading UTF-8 BOM (`EF BB BF`) if present.
fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
    if bytes.starts_with(BOM) {
        &bytes[BOM.len()..]
    } else {
        bytes
    }
}

pub fn parse_manifest(bytes: &[u8]) -> AppResult<Manifest> {
    let bytes = strip_utf8_bom(bytes);
    serde_json::from_slice(bytes).map_err(|e| {
        AppError::new("manifest_parse_failed", "无法解析 mod manifest.json")
            .with_detail(e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let raw = br#"{
      "Name": "Test Mod",
      "Author": "Ada",
      "Version": "1.2.3",
      "Description": "Hi",
      "UniqueID": "Ada.TestMod",
      "UpdateKeys": ["Nexus:1234"],
      "Dependencies": [{ "UniqueID": "Pathoschild.ContentPatcher", "IsRequired": true }]
    }"#;
        let m = parse_manifest(raw).unwrap();
        assert_eq!(m.unique_id, "Ada.TestMod");
        assert_eq!(m.update_keys, vec!["Nexus:1234"]);
    }

    #[test]
    fn parse_strips_utf8_bom() {
        let mut raw = vec![0xEF, 0xBB, 0xBF];
        raw.extend_from_slice(
            br#"{
      "Name": "BOM Mod",
      "Author": "Ada",
      "Version": "1.0.0",
      "Description": "Hi",
      "UniqueID": "Ada.BomMod"
    }"#,
        );
        let m = parse_manifest(&raw).unwrap();
        assert_eq!(m.unique_id, "Ada.BomMod");
    }

    #[test]
    fn dependency_is_required_defaults_true() {
        let raw = br#"{
      "Name": "Dep Mod",
      "Author": "Ada",
      "Version": "1.0.0",
      "Description": "Hi",
      "UniqueID": "Ada.DepMod",
      "Dependencies": [{ "UniqueID": "Pathoschild.ContentPatcher" }]
    }"#;
        let m = parse_manifest(raw).unwrap();
        assert_eq!(m.dependencies.len(), 1);
        assert!(m.dependencies[0].is_required);
        assert_eq!(m.dependencies[0].unique_id, "Pathoschild.ContentPatcher");
    }
}
