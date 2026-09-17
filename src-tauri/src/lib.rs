mod commands;
mod domain;
mod error;
mod storage;

use commands::game::{discover_paths, get_settings, save_settings, validate_paths};
use commands::mods::{
    install_from_nxm, install_from_nxm_blocking, install_mod_zip, scan_mods, set_mod_enabled,
};
use commands::nexus::{
    nexus_clear_key, nexus_endorse, nexus_set_key, nexus_status, nexus_update_mod, nexus_validate,
};
use commands::profiles::{
    apply_profile, create_profile, delete_profile, list_profiles, rename_profile,
    snapshot_current_as_profile,
};
use commands::smapi::launch_smapi;
use commands::updates::check_mod_updates;
use storage::log_util::open_log_dir;
use tauri::Emitter;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn process_nxm_url(app: &tauri::AppHandle, url: &str) {
    // Never log NXM key query params in plaintext.
    let safe_hint = url.split('?').next().unwrap_or("nxm://…").to_string();
    let _ = app.emit(
        "nxm-install-started",
        serde_json::json!({
            "hint": safe_hint,
            "message": "正在通过 NXM 下载并安装模组…",
        }),
    );

    let handle = app.clone();
    let url = url.to_string();
    let hint_for_err = safe_hint;
    tauri::async_runtime::spawn(async move {
        let outcome = tauri::async_runtime::spawn_blocking(move || install_from_nxm_blocking(url))
            .await;
        match outcome {
            Ok(Ok(entry)) => {
                let _ = handle.emit(
                    "nxm-install-result",
                    serde_json::json!({
                        "ok": true,
                        "mod": entry,
                        "message": format!("已通过 NXM 安装：{}", entry.name),
                    }),
                );
            }
            Ok(Err(e)) => {
                let _ = handle.emit(
                    "nxm-install-result",
                    serde_json::json!({
                        "ok": false,
                        "code": e.code,
                        "message": e.message,
                        "detail": e.detail,
                        "hint": hint_for_err,
                    }),
                );
            }
            Err(_) => {
                let _ = handle.emit(
                    "nxm-install-result",
                    serde_json::json!({
                        "ok": false,
                        "code": "task_join_failed",
                        "message": "NXM 安装后台任务失败",
                        "detail": null,
                        "hint": hint_for_err,
                    }),
                );
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {
            // With the deep-link feature, NXM URLs are forwarded to on_open_url.
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                // Dev / unpackaged: register nxm:// to this executable.
                if let Err(_e) = app.deep_link().register_all() {
                    // Non-fatal: packaged installs register via tauri.conf.json.
                }
            }

            #[cfg(desktop)]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        let s = url.as_str().to_string();
                        if s.to_ascii_lowercase().starts_with("nxm://") {
                            process_nxm_url(&handle, &s);
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_settings,
            save_settings,
            discover_paths,
            validate_paths,
            scan_mods,
            set_mod_enabled,
            install_mod_zip,
            install_from_nxm,
            launch_smapi,
            list_profiles,
            create_profile,
            rename_profile,
            delete_profile,
            apply_profile,
            snapshot_current_as_profile,
            check_mod_updates,
            nexus_set_key,
            nexus_clear_key,
            nexus_status,
            nexus_validate,
            nexus_endorse,
            nexus_update_mod,
            open_log_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
