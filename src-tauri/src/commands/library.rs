use serde::Serialize;

use crate::domain::game;
use crate::domain::library::{self, ImportMeta, LibraryMod, LibraryQuery, ProfileState};
use crate::domain::smapi_update::UpdateInfo;
use crate::domain::nexus::download::{self, NexusAcquire, NexusFileInfo};
use crate::error::{AppError, AppResult};
use crate::storage::log_util::log_result;
use crate::storage::paths::{apply_marker_path, library_dir, userdata_dir};
use crate::storage::profiles_store;
use crate::storage::settings;

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn prepare() -> AppResult<()> {
    let _ = library::resume_incomplete_apply(&apply_marker_path())?;
    let settings = settings::load_settings().ok();
    if let Some(settings) = settings {
        if let Ok(resolved) = game::resolve_paths(&settings) {
            library::migrate_installed_mods(
                &resolved.mods_path,
                &library_dir(),
                &userdata_dir(),
                &profiles_store::profiles_dir(),
                &now_iso(),
            )?;
            return Ok(());
        }
    }
    profiles_store::ensure_default_profile(
        &profiles_store::profiles_dir(),
        Vec::new(),
        &now_iso(),
    )?;
    Ok(())
}

fn mods_path_opt() -> Option<std::path::PathBuf> {
    let settings = settings::load_settings().ok()?;
    game::resolve_paths(&settings).ok().map(|p| p.mods_path)
}

#[tauri::command]
pub fn list_library(
    category: Option<String>,
    id: Option<String>,
    keyword: Option<String>,
) -> AppResult<Vec<LibraryMod>> {
    log_result((|| {
        prepare()?;
        library::list_mods(
            &library_dir(),
            LibraryQuery {
                category: category.as_deref(),
                id: id.as_deref(),
                keyword: keyword.as_deref(),
            },
        )
    })())
}

#[tauri::command]
pub fn delete_library_mod(id: String) -> AppResult<()> {
    log_result(library::delete_mod(
        &library_dir(),
        &profiles_store::profiles_dir(),
        &id,
    ))
}

#[tauri::command]
pub fn home_state() -> AppResult<ProfileState> {
    log_result((|| {
        prepare()?;
        let preferred = settings::load_settings()
            .ok()
            .and_then(|item| item.last_profile_id)
            .filter(|id| !id.is_empty());
        let profile = match preferred {
            Some(id) => profiles_store::load_profile_by_id(&id)
                .or_else(|_| profiles_store::load_profile_by_id("default"))?,
            None => profiles_store::load_profile_by_id("default")?,
        };
        library::profile_state(mods_path_opt().as_deref(), &library_dir(), &profile)
    })())
}

#[tauri::command]
pub fn profile_state(id: String) -> AppResult<ProfileState> {
    log_result((|| {
        prepare()?;
        let profile = profiles_store::load_profile_by_id(&id)?;
        library::profile_state(mods_path_opt().as_deref(), &library_dir(), &profile)
    })())
}

fn join_blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> impl std::future::Future<Output = AppResult<T>> {
    async move {
        tauri::async_runtime::spawn_blocking(f)
            .await
            .map_err(|_| AppError::new("task_join_failed", "后台任务失败"))?
    }
}

fn swap_zip(id_hint: &str, zip: &std::path::Path, meta: ImportMeta) -> AppResult<LibraryMod> {
    let mods = mods_path_opt();
    let imported =
        library::replace_from_zip(&library_dir(), &userdata_dir(), mods.as_deref(), zip, meta)?;
    if id_hint.is_empty() {
        return imported.into_iter().next().ok_or_else(|| {
            AppError::new("zip_no_mod", "压缩包中未找到有效的模组目录")
        });
    }
    imported
        .into_iter()
        .find(|item| item.id == id_hint)
        .ok_or_else(|| {
            AppError::new(
                "library_id_mismatch",
                "下载到的模组与本地库记录不是同一个",
            )
        })
}

#[tauri::command]
pub async fn library_add_from_nexus(
    mod_id: u32,
    category: Option<String>,
) -> AppResult<LibraryImportResult> {
    join_blocking(move || {
        log_result((|| {
            prepare()?;
            let file_id = download::list_nexus_files(mod_id)?
                .into_iter()
                .filter(|f| f.is_main)
                .max_by_key(|f| f.uploaded_timestamp)
                .map(|f| f.file_id)
                .ok_or_else(|| AppError::new("nexus_no_main_file", "未找到可用的主文件"))?;
            match download::acquire_nexus_file(mod_id, file_id)? {
                NexusAcquire::Direct(cdn) => {
                    let zip = download::download_url_to_temp_zip(&cdn)?;
                    let mods = mods_path_opt();
                    let imported = library::replace_from_zip(
                        &library_dir(),
                        &userdata_dir(),
                        mods.as_deref(),
                        &zip,
                        ImportMeta {
                            category,
                            nexus_mod_id: Some(mod_id),
                            nexus_file_id: Some(file_id),
                        },
                    )?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        AppError::new("zip_no_mod", "压缩包中未找到有效的模组目录")
                    })?;
                    let _ = std::fs::remove_file(&zip);
                    Ok(LibraryImportResult {
                        imported: Some(imported),
                        browser_url: None,
                    })
                }
                NexusAcquire::ManualPage(url) => Ok(LibraryImportResult {
                    imported: None,
                    browser_url: Some(url),
                }),
            }
        })())
    })
    .await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryImportResult {
    pub imported: Option<LibraryMod>,
    pub browser_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryUpdateResult {
    pub updated: Option<LibraryMod>,
    pub browser_url: Option<String>,
}

#[tauri::command]
pub async fn library_update(id: String) -> AppResult<LibraryUpdateResult> {
    join_blocking(move || {
        log_result((|| {
            let current = library::load_mod(&library_dir(), &id)?;
            let mod_id = current.nexus_mod_id.ok_or_else(|| {
                AppError::new(
                    "nexus_id_missing",
                    "这个本地模组没有 Nexus 编号，不能在线更新",
                )
            })?;
            let file_id = download::list_nexus_files(mod_id)?
                .into_iter()
                .filter(|f| f.is_main)
                .max_by_key(|f| f.uploaded_timestamp)
                .map(|f| f.file_id)
                .ok_or_else(|| AppError::new("nexus_no_main_file", "未找到可用的主文件"))?;
            match download::acquire_nexus_file(mod_id, file_id)? {
                NexusAcquire::Direct(cdn) => {
                    let zip = download::download_url_to_temp_zip(&cdn)?;
                    let updated = swap_zip(
                        &id,
                        &zip,
                        ImportMeta {
                            category: current.category,
                            nexus_mod_id: Some(mod_id),
                            nexus_file_id: Some(file_id),
                        },
                    );
                    let _ = std::fs::remove_file(&zip);
                    Ok(LibraryUpdateResult {
                        updated: Some(updated?),
                        browser_url: None,
                    })
                }
                NexusAcquire::ManualPage(url) => Ok(LibraryUpdateResult {
                    updated: None,
                    browser_url: Some(url),
                }),
            }
        })())
    })
    .await
}

#[tauri::command]
pub async fn library_nexus_files(id: String) -> AppResult<Vec<NexusFileInfo>> {
    join_blocking(move || {
        log_result((|| {
            let current = library::load_mod(&library_dir(), &id)?;
            let mod_id = current.nexus_mod_id.ok_or_else(|| {
                AppError::new("nexus_id_missing", "这个本地模组没有 Nexus 编号，不能降级")
            })?;
            download::list_nexus_files(mod_id)
        })())
    })
    .await
}

#[tauri::command]
pub async fn library_downgrade(id: String, file_id: u64) -> AppResult<LibraryMod> {
    join_blocking(move || {
        log_result((|| {
            let current = library::load_mod(&library_dir(), &id)?;
            let mod_id = current.nexus_mod_id.ok_or_else(|| {
                AppError::new("nexus_id_missing", "这个本地模组没有 Nexus 编号，不能降级")
            })?;
            let cdn = download::fetch_premium_download_link(mod_id, file_id)?;
            let zip = download::download_url_to_temp_zip(&cdn)?;
            let result = swap_zip(
                &id,
                &zip,
                ImportMeta {
                    category: current.category,
                    nexus_mod_id: Some(mod_id),
                    nexus_file_id: Some(file_id),
                },
            );
            let _ = std::fs::remove_file(&zip);
            result
        })())
    })
    .await
}

#[tauri::command]
pub async fn library_check_updates() -> AppResult<Vec<UpdateInfo>> {
    join_blocking(|| {
        log_result((|| {
            prepare()?;
            let entries = library::entries_for_updates(&library_dir())?;
            crate::domain::smapi_update::check_updates(&entries)
        })())
    })
    .await
}
