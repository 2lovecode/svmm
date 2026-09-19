use serde::Deserialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Manifest {
    #[serde(rename = "UniqueID", alias = "UniqueId")]
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
    #[serde(rename = "UniqueID", alias = "UniqueId")]
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
    let text = std::str::from_utf8(bytes).map_err(|e| {
        AppError::new("manifest_parse_failed", "无法解析 mod manifest.json")
            .with_detail(e.to_string())
    })?;
    // SMAPI (Json.NET) accepts comments and trailing commas. ModManifestBuilder
    // writes a block comment on line 2 of generated manifests.
    let strict = smapi_json_to_strict(text);
    serde_json::from_str(&strict).map_err(|e| {
        AppError::new("manifest_parse_failed", "无法解析 mod manifest.json")
            .with_detail(e.to_string())
    })
}

/// Turn SMAPI's lenient manifest JSON into strict JSON for serde_json.
fn smapi_json_to_strict(input: &str) -> String {
    let without_comments = strip_json_comments(input);
    strip_trailing_commas(&without_comments)
}

fn strip_json_comments(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            i += 2;
            while i < chars.len() && chars[i] != '\n' && chars[i] != '\r' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = if i + 1 < chars.len() { i + 2 } else { chars.len() };
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

fn strip_trailing_commas(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
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

    #[test]
    fn parse_accepts_smapi_unique_id_spelling() {
        let raw = br#"{
      "Name": "Console Commands",
      "Author": "SMAPI",
      "Version": "4.5.2",
      "Description": "bundled",
      "UniqueId": "SMAPI.ConsoleCommands"
    }"#;
        let m = parse_manifest(raw).unwrap();
        assert_eq!(m.unique_id, "SMAPI.ConsoleCommands");
        assert_eq!(m.version, "4.5.2");
    }

    #[test]
    fn parse_mod_manifest_builder_comment_and_trailing_comma() {
        let raw = br#"{
  /*
  | This file is automatically generated by ModManifestBuilder
  | when the project is compiled.
  |
  | Do not change this file directly.
   */
  "Name": "Generic Mod Config Menu",
  "Author": "spacechase0",
  "Version": "1.16.0",
  "Description": "Adds an in-game UI to edit other mods' config options.",
  "UniqueID": "spacechase0.GenericModConfigMenu",
  "EntryDll": "GenericModConfigMenu.dll",
  "MinimumApiVersion": "4.1.0",
  "UpdateKeys": [ "Nexus:5098", ],
  "Dependencies": [
    { "UniqueID": "Pathoschild.ContentPatcher", "MinimumVersion": "1.0.0", }
  ],
}"#;
        let m = parse_manifest(raw).unwrap();
        assert_eq!(m.unique_id, "spacechase0.GenericModConfigMenu");
        assert_eq!(m.name, "Generic Mod Config Menu");
        assert_eq!(m.version, "1.16.0");
        assert_eq!(m.update_keys, vec!["Nexus:5098"]);
        assert_eq!(m.dependencies[0].unique_id, "Pathoschild.ContentPatcher");
    }

    #[test]
    fn comments_inside_strings_are_kept() {
        let raw = br#"{
      // header
      "Name": "http://example.com /* no */",
      "Author": "Ada",
      "Version": "1.0.0",
      "Description": "line // still here",
      "UniqueID": "Ada.Url"
    }"#;
        let m = parse_manifest(raw).unwrap();
        assert_eq!(m.name, "http://example.com /* no */");
        assert_eq!(m.description, "line // still here");
        assert_eq!(m.unique_id, "Ada.Url");
    }
}
