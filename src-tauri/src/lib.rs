mod commands;
mod domain;
mod error;
mod storage;

use commands::game::{discover_paths, get_settings, save_settings, validate_paths};
use commands::mods::{scan_mods, set_mod_enabled};
use commands::profiles::{
    apply_profile, create_profile, delete_profile, list_profiles, rename_profile,
    snapshot_current_as_profile,
};
use commands::smapi::launch_smapi;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_settings,
            save_settings,
            discover_paths,
            validate_paths,
            scan_mods,
            set_mod_enabled,
            launch_smapi,
            list_profiles,
            create_profile,
            rename_profile,
            delete_profile,
            apply_profile,
            snapshot_current_as_profile
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}