/** IPC DTOs — camelCase to match Rust `#[serde(rename_all = "camelCase")]`. */

export interface ModDependency {
  uniqueId: string;
  isRequired: boolean;
}

export interface ModEntry {
  id: string;
  name: string;
  author: string;
  version: string;
  description: string;
  folderPath: string;
  enabled: boolean;
  minimumApiVersion: string | null;
  updateKeys: string[];
  dependencies: ModDependency[];
  status: string;
}

export interface Settings {
  gamePath: string | null;
  smapiPath: string | null;
  modsPath: string | null;
  lastProfileId: string | null;
  theme: string;
  language: string;
  checkUpdatesOnStartup: boolean;
  downloadProxy: string | null;
}

export interface GamePaths {
  gamePath: string;
  smapiPath: string;
  modsPath: string;
}

export interface SmapiStatus {
  installed: boolean;
  gameFound: boolean;
  gamePath: string | null;
  smapiPath: string | null;
}

export interface SmapiInstallReport {
  version: string;
  smapiPath: string;
}

export interface SmapiInstallProgress {
  phase: string;
  message: string;
  received: number;
  total: number | null;
  percent: number | null;
}

export interface Profile {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  modIds: string[];
}

export interface ApplyReport {
  deployed: number;
  removed: number;
  errors: string[];
}

export interface LibraryMod {
  id: string;
  name: string;
  author: string;
  version: string;
  description: string;
  idFromManifest: boolean;
  category: string | null;
  nexusModId: number | null;
  nexusFileId: number | null;
  folderName: string;
  files: { path: string; sha256: string }[];
}

export interface ProfileState {
  profile: Profile;
  mods: LibraryMod[];
  applied: boolean;
}

export interface NexusFileInfo {
  fileId: number;
  name: string;
  version: string;
  categoryName: string;
  uploadedTimestamp: number;
  isMain: boolean;
}

export interface UpdateInfo {
  id: string;
  status: string;
  suggestedVersion: string | null;
  errorReason: string | null;
}

export interface NexusStatus {
  hasKey: boolean;
}

export interface NexusUser {
  name: string;
  isPremium: boolean;
  isSupporter: boolean;
}

export interface NexusCategory {
  name: string;
  count: number;
}

export interface NexusCatalogMod {
  modId: number;
  name: string;
  author: string;
  summary: string;
  version: string;
  category: string;
  downloads: number;
  endorsements: number;
  thumbnailUrl: string | null;
  libraryStatus: string;
}

export interface NexusBrowsePage {
  page: number;
  pageSize: number;
  total: number;
  mods: NexusCatalogMod[];
}

export interface AppError {
  code: string;
  message: string;
  detail?: string | null;
}

export function defaultSettings(): Settings {
  return {
    gamePath: null,
    smapiPath: null,
    modsPath: null,
    lastProfileId: null,
    theme: "system",
    language: "zh-CN",
    checkUpdatesOnStartup: true,
    downloadProxy: null,
  };
}

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value &&
    typeof (value as AppError).code === "string" &&
    typeof (value as AppError).message === "string"
  );
}

export function formatAppError(err: unknown): string {
  if (isAppError(err)) {
    return err.detail ? `${err.message}（${err.detail}）` : err.message;
  }
  if (err instanceof Error) {
    return err.message;
  }
  return String(err);
}
