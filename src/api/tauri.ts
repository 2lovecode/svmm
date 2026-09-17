import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyReport,
  GamePaths,
  ModEntry,
  Profile,
  Settings,
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
  enabledModIds?: string[] | null,
): Promise<Profile> {
  return invoke<Profile>("create_profile", { name, enabledModIds: enabledModIds ?? null });
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

export function snapshotCurrentAsProfile(name: string): Promise<Profile> {
  return invoke<Profile>("snapshot_current_as_profile", { name });
}

export function checkModUpdates(): Promise<UpdateInfo[]> {
  return invoke<UpdateInfo[]>("check_mod_updates");
}
