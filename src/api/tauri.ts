import { invoke } from "@tauri-apps/api/core";
import type { GamePaths, ModEntry, Settings } from "../types/mod";

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
