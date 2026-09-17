use serde::Serialize;

use crate::domain::nexus::{self, NexusUser};
use crate::error::AppResult;
use crate::storage::secure_key;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NexusStatus {
    pub has_key: bool,
}

#[tauri::command]
pub fn nexus_set_key(key: String) -> AppResult<()> {
    secure_key::set_nexus_api_key(&key)
}

#[tauri::command]
pub fn nexus_clear_key() -> AppResult<()> {
    secure_key::clear_nexus_api_key()
}

#[tauri::command]
pub fn nexus_status() -> AppResult<NexusStatus> {
    Ok(NexusStatus {
        has_key: secure_key::has_nexus_api_key(),
    })
}

#[tauri::command]
pub fn nexus_validate() -> AppResult<NexusUser> {
    nexus::validate_nexus_api_key()
}
