import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyReport,
  GamePaths,
  LibraryImportResult,
  LibraryMod,
  LibraryUpdateResult,
  ModEntry,
  NexusBrowsePage,
  NexusCategory,
  NexusFileInfo,
  NexusStatus,
  NexusUser,
  Profile,
  ProfileState,
  Settings,
  SmapiInstallReport,
  SmapiStatus,
  UpdateInfo,
} from "../types/mod";

export function scanMods(): Promise<ModEntry[]> {
  return invoke<ModEntry[]>("scan_mods");
}

export function setModEnabled(
  folderPath: string,
  enabled: boolean,
): Promise<ModEntry> {
  return invoke<ModEntry>("set_mod_enabled", { folderPath, enabled });
}

export function launchSmapi(): Promise<void> {
  return invoke<void>("launch_smapi");
}

export function smapiStatus(): Promise<SmapiStatus> {
  return invoke<SmapiStatus>("smapi_status");
}

export function installSmapi(): Promise<SmapiInstallReport> {
  return invoke<SmapiInstallReport>("install_smapi");
}

export function uninstallSmapi(): Promise<void> {
  return invoke<void>("uninstall_smapi");
}

export function smapiLatestVersion(): Promise<string> {
  return invoke<string>("smapi_latest_version");
}

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function saveSettings(settings: Settings): Promise<void> {
  return invoke<void>("save_settings", { settings });
}

export function discoverPaths(): Promise<GamePaths | null> {
  return invoke<GamePaths | null>("discover_paths");
}

export function listProfiles(): Promise<Profile[]> {
  return invoke<Profile[]>("list_profiles");
}

export function createProfile(
  name: string,
  modIds?: string[] | null,
): Promise<Profile> {
  return invoke<Profile>("create_profile", { name, modIds: modIds ?? null });
}

export function renameProfile(id: string, name: string): Promise<Profile> {
  return invoke<Profile>("rename_profile", { id, name });
}

export function deleteProfile(id: string): Promise<void> {
  return invoke<void>("delete_profile", { id });
}

export function applyProfile(id: string): Promise<ApplyReport> {
  return invoke<ApplyReport>("apply_profile", { id });
}

export function addProfileMod(id: string, modId: string): Promise<Profile> {
  return invoke<Profile>("add_profile_mod", { id, modId });
}

export function removeProfileMod(id: string, modId: string): Promise<Profile> {
  return invoke<Profile>("remove_profile_mod", { id, modId });
}

export function profileDetail(id: string): Promise<ProfileState> {
  return invoke<ProfileState>("profile_detail", { id });
}

export function homeState(): Promise<ProfileState> {
  return invoke<ProfileState>("home_state");
}

export function listLibrary(filter?: {
  category?: string | null;
  id?: string | null;
  keyword?: string | null;
}): Promise<LibraryMod[]> {
  return invoke<LibraryMod[]>("list_library", {
    category: filter?.category ?? null,
    id: filter?.id ?? null,
    keyword: filter?.keyword ?? null,
  });
}

export function deleteLibraryMod(id: string): Promise<void> {
  return invoke<void>("delete_library_mod", { id });
}

export function libraryAddFromNexus(
  modId: number,
  category?: string | null,
): Promise<LibraryImportResult> {
  return invoke<LibraryImportResult>("library_add_from_nexus", {
    modId,
    category: category ?? null,
  });
}

export function libraryUpdate(id: string): Promise<LibraryUpdateResult> {
  return invoke<LibraryUpdateResult>("library_update", { id });
}

export function libraryCheckUpdates(): Promise<UpdateInfo[]> {
  return invoke<UpdateInfo[]>("library_check_updates");
}

export function libraryNexusFiles(id: string): Promise<NexusFileInfo[]> {
  return invoke<NexusFileInfo[]>("library_nexus_files", { id });
}

export function libraryDowngrade(id: string, fileId: number): Promise<LibraryMod> {
  return invoke<LibraryMod>("library_downgrade", { id, fileId });
}

export function snapshotCurrentAsProfile(name: string): Promise<Profile> {
  return invoke<Profile>("snapshot_current_as_profile", { name });
}

export function checkModUpdates(): Promise<UpdateInfo[]> {
  return invoke<UpdateInfo[]>("check_mod_updates");
}

export function nexusSetKey(key: string): Promise<void> {
  return invoke<void>("nexus_set_key", { key });
}

export function nexusClearKey(): Promise<void> {
  return invoke<void>("nexus_clear_key");
}

export function nexusStatus(): Promise<NexusStatus> {
  return invoke<NexusStatus>("nexus_status");
}

export function nexusValidate(): Promise<NexusUser> {
  return invoke<NexusUser>("nexus_validate");
}

export function nexusEndorse(
  modId: number,
  version?: string | null,
): Promise<void> {
  return invoke<void>("nexus_endorse", {
    modId,
    version: version ?? null,
  });
}

export function nexusUpdateMod(folderPath: string): Promise<ModEntry> {
  return invoke<ModEntry>("nexus_update_mod", { folderPath });
}

export function nexusCategories(): Promise<NexusCategory[]> {
  return invoke<NexusCategory[]>("nexus_list_categories");
}

export function nexusBrowseMods(
  page: number,
  category: string | null,
  keyword: string | null,
  modId: number | null,
): Promise<NexusBrowsePage> {
  return invoke<NexusBrowsePage>("nexus_browse_mods", {
    page,
    category,
    keyword,
    modId,
  });
}

export function installModZip(path: string): Promise<LibraryMod> {
  return invoke<LibraryMod>("install_mod_zip", { path });
}

export function installFromNxm(url: string): Promise<LibraryMod> {
  return invoke<LibraryMod>("install_from_nxm", { url });
}

export function openLogDir(): Promise<void> {
  return invoke<void>("open_log_dir");
}
