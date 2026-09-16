mod commands;
mod domain;
mod error;
mod storage;

use commands::game::{discover_paths, get_settings, save_settings, validate_paths};
use commands::mods::{scan_mods, set_mod_enabled};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_settings,
            save_settings,
            discover_paths,
            validate_paths,
            scan_mods,
            set_mod_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}