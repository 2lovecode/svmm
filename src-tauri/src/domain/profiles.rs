//! Profile domain: diff and apply enablement sets by UniqueID.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::mods::enable::set_mod_enabled;
use crate::domain::mods::scan::ModEntry;
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub enabled_mod_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReport {
    pub enabled: usize,
    pub disabled: usize,
    pub errors: Vec<String>,
}

/// Diff current enabled UniqueIDs against a target set.
/// Returns `(to_enable, to_disable)`.
pub fn profile_diff(current_enabled: &[String], target: &[String]) -> (Vec<String>, Vec<String>) {
    let current: HashSet<&str> = current_enabled.iter().map(String::as_str).collect();
    let want: HashSet<&str> = target.iter().map(String::as_str).collect();

    let mut to_enable: Vec<String> = want
        .difference(&current)
        .map(|s| (*s).to_string())
        .collect();
    let mut to_disable: Vec<String> = current
        .difference(&want)
        .map(|s| (*s).to_string())
        .collect();
    to_enable.sort();
    to_disable.sort();
    (to_enable, to_disable)
}

/// Apply a profile to the Mods tree using UniqueID sets vs current entries.
pub fn apply_profile(
    mods_path: &Path,
    profile: &Profile,
    current_entries: &[ModEntry],
) -> AppResult<ApplyReport> {
    let current_enabled: Vec<String> = current_entries
        .iter()
        .filter(|e| e.enabled)
        .map(|e| e.id.clone())
        .collect();

    let (to_enable, to_disable) = profile_diff(&current_enabled, &profile.enabled_mod_ids);

    let by_id: std::collections::HashMap<&str, &ModEntry> = current_entries
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect();

    let mut enabled = 0usize;
    let mut disabled = 0usize;
    let mut errors = Vec::new();

    for id in &to_enable {
        match by_id.get(id.as_str()) {
            Some(entry) => match set_mod_enabled(mods_path, &entry.folder_path, true) {
                Ok(_) => enabled += 1,
                Err(e) => errors.push(format!("{}: {}", id, e)),
            },
            None => errors.push(format!("{}: mod not found in current scan", id)),
        }
    }

    for id in &to_disable {
        match by_id.get(id.as_str()) {
            Some(entry) => match set_mod_enabled(mods_path, &entry.folder_path, false) {
                Ok(_) => disabled += 1,
                Err(e) => errors.push(format!("{}: {}", id, e)),
            },
            None => errors.push(format!("{}: mod not found in current scan", id)),
        }
    }

    Ok(ApplyReport {
        enabled,
        disabled,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mods::scan::scan_mods;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn diff_enables_and_disables() {
        let (en, dis) = profile_diff(&["A".into(), "B".into()], &["B".into(), "C".into()]);
        assert_eq!(en, vec!["C".to_string()]);
        assert_eq!(dis, vec!["A".to_string()]);
    }

    fn write_mod(mods: &Path, folder: &str, unique_id: &str) {
        let dir = mods.join(folder);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("manifest.json"),
            format!(
                r#"{{
  "Name": "{unique_id}",
  "Author": "Test",
  "Version": "1.0.0",
  "Description": "Hi",
  "UniqueID": "{unique_id}"
}}"#
            ),
        )
        .unwrap();
    }

    fn make_two_mods_fixture() -> PathBuf {
        let mods = std::env::temp_dir().join(format!(
            "svmm-profiles-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&mods);
        fs::create_dir_all(&mods).unwrap();
        write_mod(&mods, "Mod.A", "Mod.A");
        write_mod(&mods, "Mod.B", "Mod.B");
        mods
    }

    #[test]
    fn apply_profile_enables_and_disables() {
        let mods = make_two_mods_fixture();
        let entries = scan_mods(&mods).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.enabled));

        let profile = Profile {
            id: "p1".into(),
            name: "only-b".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            enabled_mod_ids: vec!["Mod.B".into()],
        };

        let report = apply_profile(&mods, &profile, &entries).unwrap();
        assert_eq!(report.enabled, 0);
        assert_eq!(report.disabled, 1);
        assert!(report.errors.is_empty());

        let after = scan_mods(&mods).unwrap();
        let a = after.iter().find(|e| e.id == "Mod.A").unwrap();
        let b = after.iter().find(|e| e.id == "Mod.B").unwrap();
        assert!(!a.enabled);
        assert!(b.enabled);

        let _ = fs::remove_dir_all(&mods);
    }
}
