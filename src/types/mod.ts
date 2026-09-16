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
}

export interface GamePaths {
  gamePath: string;
  smapiPath: string;
  modsPath: string;
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
