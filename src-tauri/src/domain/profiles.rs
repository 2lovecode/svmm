//! Profile groups: a named list of library UniqueIDs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    /// UniqueIDs selected from the local library. Not an enable/disable set.
    #[serde(rename = "modIds", alias = "enabledModIds", default)]
    pub mod_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReport {
    pub deployed: usize,
    pub removed: usize,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_legacy_enabled_mod_ids() {
        let raw = r#"{
            "id": "default",
            "name": "default",
            "createdAt": "t",
            "updatedAt": "t",
            "enabledModIds": ["A", "B"]
        }"#;
        let profile: Profile = serde_json::from_str(raw).unwrap();
        assert_eq!(profile.mod_ids, vec!["A".to_string(), "B".to_string()]);
    }
}
