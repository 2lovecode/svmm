# SVMM Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build SVMM, a Windows Tauri 2 + Vue 3 mod manager for Stardew Valley with Stardrop-class features (local management, profiles, update checks, Nexus Mods).

**Architecture:** Rust domain modules behind typed Tauri IPC; Vue UI is presentation-only. Enable/disable is non-destructive via SMAPI’s “ignore folders starting with `.`” rule. Config lives in `%APPDATA%/svmm/`.

**Tech Stack:** Tauri 2, Vue 3, TypeScript, Vite, Rust 2021, serde/serde_json, thiserror, reqwest, zip, tauri-plugin-dialog, tauri-plugin-process, tauri-plugin-deep-link (NXM), keyring (Nexus API key).

**Spec:** `docs/superpowers/specs/2026-09-16-svmm-design.md`

## Global Constraints

- Platform: Windows first (no macOS/Linux work in this plan).
- Product name: SVMM; UI language default `zh-CN`.
- Non-destructive: never delete the user’s Mods tree to “disable”; use `.` folder prefix.
- Nexus API key: never log plaintext; store via `keyring` service name `svmm`, account `nexus_api_key`.
- Zip install: reject entries that escape `Mods/` (`..` or absolute paths).
- `nxm://` handler: only `stardewvalley` game domain.
- Enable/disable convention (locked): **disable** = rename `Mods/SomeMod` → `Mods/.SomeMod`; **enable** = reverse. If name already starts with `.`, treat as disabled.
- Update API (locked): `POST https://smapi.io/api/v3.0/mods` with JSON body listing mods by update keys / unique IDs (SMAPI public API).
- Commits: small, frequent; message style `feat:` / `fix:` / `test:` / `chore:`.

---

## File Structure

```
svmm/
  package.json                 # Vue/Vite + Tauri scripts
  src/
    main.ts
    App.vue
    router/index.ts
    styles/app.css
    types/mod.ts               # mirrors Rust DTOs
    api/tauri.ts               # invoke wrappers
    stores/mods.ts
    stores/settings.ts
    stores/profiles.ts
    components/
      ModTable.vue
      StatusBar.vue
      TopBar.vue
    views/
      HomeView.vue
      SettingsView.vue
      ProfilesView.vue
  src-tauri/
    Cargo.toml
    tauri.conf.json
    capabilities/default.json
    src/
      lib.rs
      main.rs
      error.rs
      commands/
        mod.rs
        game.rs
        mods.rs
        profiles.rs
        smapi.rs
        nexus.rs
      domain/
        game.rs
        mods/
          mod.rs
          manifest.rs
          enable.rs
          scan.rs
          install.rs
        profiles.rs
        smapi_update.rs
        nexus/
          mod.rs
          client.rs
          download.rs
      storage/
        paths.rs
        settings.rs
        profiles_store.rs
        secure_key.rs
        log_util.rs
  docs/…                       # existing specs/plans (do not delete)
```

---

### Task 1: Scaffold Tauri 2 + Vue 3 + TypeScript

**Files:**
- Create: project root scaffold via `create-tauri-app` (preserve existing `docs/`)
- Modify: `package.json` name → `svmm`; `src-tauri/tauri.conf.json` productName → `SVMM`
- Test: manual `npm run tauri build` not required yet; `npm run dev` / `cargo check` smoke

**Interfaces:**
- Consumes: none
- Produces: runnable empty Tauri+Vue app in repo root

- [ ] **Step 1: Scaffold into repo**

Repo already has `docs/` and git history. From repo root (PowerShell):

```powershell
npm create tauri-app@latest . -- --template vue-ts --manager npm --yes
```

If the tool refuses a non-empty directory: scaffold into `_scaffold`, move files up (keep `docs/`), delete `_scaffold`.

- [ ] **Step 2: Align names**

Set `package.json` `"name": "svmm"`. In `src-tauri/tauri.conf.json` set `"productName": "SVMM"` and identifier `com.svmm.app`.

- [ ] **Step 3: Smoke check**

```powershell
npm install
cd src-tauri; cargo check; cd ..
```

Expected: cargo check succeeds.

- [ ] **Step 4: Commit**

```powershell
git add -A
git commit -m "chore: scaffold Tauri 2 Vue TypeScript app for SVMM"
```

---

### Task 2: AppError + app data paths + settings storage

**Files:**
- Create: `src-tauri/src/error.rs`
- Create: `src-tauri/src/storage/paths.rs`
- Create: `src-tauri/src/storage/settings.rs`
- Create: `src-tauri/src/storage/mod.rs`
- Modify: `src-tauri/Cargo.toml` (serde, serde_json, thiserror, directories)
- Modify: `src-tauri/src/lib.rs` (mod declarations)
- Test: `src-tauri/src/storage/settings.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: none
- Produces:
  - `AppError { code: String, message: String, detail: Option<String> }` with `Serialize`
  - `pub type AppResult<T> = Result<T, AppError>;`
  - `Settings { game_path, smapi_path, mods_path, last_profile_id, theme, language, check_updates_on_startup }`
  - `fn app_data_dir() -> PathBuf` → `%APPDATA%/svmm`
  - `fn load_settings() -> AppResult<Settings>`
  - `fn save_settings(&Settings) -> AppResult<()>`

- [ ] **Step 1: Write failing tests for settings round-trip**

In `settings.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn settings_round_trip_in_temp_dir() {
        let dir = std::env::temp_dir().join(format!("svmm-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let s = Settings {
            game_path: Some(r"C:\Games\Stardew Valley".into()),
            smapi_path: None,
            mods_path: None,
            last_profile_id: Some("default".into()),
            theme: "system".into(),
            language: "zh-CN".into(),
            check_updates_on_startup: true,
        };
        save_settings_to(&path, &s).unwrap();
        let loaded = load_settings_from(&path).unwrap();
        assert_eq!(loaded.game_path, s.game_path);
        assert_eq!(loaded.language, "zh-CN");
        let _ = fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 2: Run test — expect fail**

```powershell
cd src-tauri
cargo test settings_round_trip_in_temp_dir -- --nocapture
```

Expected: compile error / missing symbols.

- [ ] **Step 3: Implement error + paths + settings**

```rust
// error.rs (sketch)
#[derive(Debug, serde::Serialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
}
impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self { /* ... */ }
}
```

`load_settings` creates default `Settings` if file missing. Defaults: `theme=system`, `language=zh-CN`, `check_updates_on_startup=true`, paths `None`.

- [ ] **Step 4: Run test — expect pass**

```powershell
cargo test settings_round_trip_in_temp_dir
```

- [ ] **Step 5: Commit**

```powershell
git add src-tauri
git commit -m "feat: add AppError and settings storage"
```

---

### Task 3: Game / SMAPI / Mods path discovery

**Files:**
- Create: `src-tauri/src/domain/mod.rs`
- Create: `src-tauri/src/domain/game.rs`
- Create: `src-tauri/src/commands/game.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `lib.rs` (register commands)
- Test: unit tests with fake directory trees in temp

**Interfaces:**
- Consumes: `Settings`, `AppResult`
- Produces:
  - `GamePaths { game_path: PathBuf, smapi_path: PathBuf, mods_path: PathBuf }`
  - `fn discover_game_paths() -> AppResult<Option<GamePaths>>`
  - `fn resolve_paths(settings: &Settings) -> AppResult<GamePaths>` (manual overrides win)
  - `fn validate_paths(paths: &GamePaths) -> AppResult<()>` — requires `StardewModdingAPI.exe` + `Mods/` dir
  - Commands: `get_settings`, `save_settings`, `discover_paths`, `validate_paths`

- [ ] **Step 1: Failing test — validate_paths**

```rust
#[test]
fn validate_requires_smapi_and_mods() {
    let root = temp_game_root(); // helper creates StardewModdingAPI.exe empty file + Mods/
    let paths = GamePaths {
        game_path: root.clone(),
        smapi_path: root.join("StardewModdingAPI.exe"),
        mods_path: root.join("Mods"),
    };
    assert!(validate_paths(&paths).is_ok());
}
```

- [ ] **Step 2: Run — expect fail; implement validate + discover**

Discovery candidates (exist-check only):

- `%ProgramFiles(x86)%\Steam\steamapps\common\Stardew Valley`
- `%ProgramFiles%\Steam\steamapps\common\Stardew Valley`
- `%LOCALAPPDATA%\XboxGames\Stardew Valley\Content` (best-effort)
- GOG: `%ProgramFiles(x86)%\GOG Galaxy\Games\Stardew Valley`

`smapi_path = game_path.join("StardewModdingAPI.exe")`, `mods_path = game_path.join("Mods")`.

- [ ] **Step 3: Wire commands + `cargo test`**

- [ ] **Step 4: Commit**

```powershell
git commit -m "feat: discover and validate Stardew/SMAPI paths"
```

---

### Task 4: Manifest parsing

**Files:**
- Create: `src-tauri/src/domain/mods/manifest.rs`
- Create: `src-tauri/src/domain/mods/mod.rs`
- Test: `manifest.rs` tests with sample JSON strings

**Interfaces:**
- Consumes: none
- Produces:
  - `Manifest { unique_id, name, author, version, description, minimum_api_version, update_keys, dependencies }`
  - `fn parse_manifest(bytes: &[u8]) -> AppResult<Manifest>`
  - SMAPI JSON is often lenient (BOM, trailing commas not required — use serde; strip UTF-8 BOM)

- [ ] **Step 1: Failing test**

```rust
#[test]
fn parse_minimal_manifest() {
    let raw = br#"{
      "Name": "Test Mod",
      "Author": "Ada",
      "Version": "1.2.3",
      "Description": "Hi",
      "UniqueID": "Ada.TestMod",
      "UpdateKeys": ["Nexus:1234"],
      "Dependencies": [{ "UniqueID": "Pathoschild.ContentPatcher", "IsRequired": true }]
    }"#;
    let m = parse_manifest(raw).unwrap();
    assert_eq!(m.unique_id, "Ada.TestMod");
    assert_eq!(m.update_keys, vec!["Nexus:1234"]);
}
```

Use `#[serde(rename_all = "PascalCase")]` on structs. Dependency: `UniqueID`, `IsRequired` (default true if missing).

- [ ] **Step 2: Implement until pass; commit**

```powershell
git commit -m "feat: parse SMAPI manifest.json"
```

---

### Task 5: Mod scan + enable/disable

**Files:**
- Create: `src-tauri/src/domain/mods/scan.rs`
- Create: `src-tauri/src/domain/mods/enable.rs`
- Modify: `src-tauri/src/domain/mods/mod.rs`
- Create: `src-tauri/src/commands/mods.rs`
- Test: temp Mods tree integration tests

**Interfaces:**
- Consumes: `parse_manifest`, `GamePaths`
- Produces:
  - `ModEntry { id, name, author, version, description, folder_path, enabled, minimum_api_version, update_keys, dependencies, status }`
  - `fn scan_mods(mods_path: &Path) -> AppResult<Vec<ModEntry>>`
  - `fn set_mod_enabled(mods_path: &Path, folder_path: &str, enabled: bool) -> AppResult<ModEntry>`
  - Commands: `scan_mods`, `set_mod_enabled`

**Scan rules:**
- Walk immediate child directories of `Mods/` (and one level of nested content-pack parents if folder has no manifest but children do — **v1: scan all directories that contain a `manifest.json` anywhere under `Mods/`, using the directory that contains the manifest as the mod root**).
- `enabled = !folder_name.starts_with('.')`
- `id = UniqueID` or relative path if missing manifest → `status = missing_manifest`, name = folder name
- Skip `Mods/.` system entries appropriately

**Enable/disable:**

```rust
pub fn set_mod_enabled(mods_path: &Path, relative: &str, enabled: bool) -> AppResult<PathBuf> {
    // relative is path under Mods, e.g. ".Ada.Test" or "Ada.Test" or "Nested/Pack"
    // Rename leaf folder to add/remove leading '.'
}
```

- [ ] **Step 1: Integration test**

```rust
#[test]
fn scan_and_toggle_mod() {
    let mods = make_mods_fixture(); // Ada.Test with manifest
    let list = scan_mods(&mods).unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].enabled);
    set_mod_enabled(&mods, &list[0].folder_path, false).unwrap();
    let list2 = scan_mods(&mods).unwrap();
    assert!(!list2[0].enabled);
    assert!(list2[0].folder_path.contains(".Ada") || list2[0].folder_path.starts_with('.'));
}
```

- [ ] **Step 2: Implement scan + enable; tests pass**

- [ ] **Step 3: Register commands; commit**

```powershell
git commit -m "feat: scan mods and toggle enable via dot-prefix"
```

---

### Task 6: Launch SMAPI command

**Files:**
- Create: `src-tauri/src/commands/smapi.rs`
- Modify: `src-tauri/Cargo.toml` / capabilities for process shell
- Test: unit test that builds `Command` args (do not require real game in CI)

**Interfaces:**
- Produces: `fn launch_smapi(smapi_path: &Path, game_path: &Path) -> AppResult<()>`
- Command: `launch_smapi`

```rust
std::process::Command::new(smapi_path)
    .current_dir(game_path)
    .spawn()
    .map_err(|e| AppError::new("launch_failed", e.to_string()))?;
```

- [ ] **Step 1: Implement + smoke `cargo test`; commit**

```powershell
git commit -m "feat: launch StardewModdingAPI"
```

---

### Task 7: Vue types, API wrappers, main list UI

**Files:**
- Create: `src/types/mod.ts`, `src/api/tauri.ts`, `src/stores/mods.ts`, `src/stores/settings.ts`
- Create: `src/components/TopBar.vue`, `ModTable.vue`, `StatusBar.vue`
- Create: `src/views/HomeView.vue`, `SettingsView.vue`
- Create: `src/router/index.ts`
- Modify: `src/App.vue`, `src/main.ts`, `src/styles/app.css`
- Add dependency: `vue-router`, `pinia` (or lightweight reactive stores — **use Pinia**)

**Interfaces:**
- TS `ModEntry` / `Settings` mirror Rust serde field names (`snake_case` from Rust — configure serde `rename_all = "camelCase"` on DTOs **or** use snake_case in TS; **lock: Rust DTOs use `#[serde(rename_all = "camelCase")]` for IPC**).

Commands used:

```ts
invoke<ModEntry[]>('scan_mods')
invoke<ModEntry>('set_mod_enabled', { folderPath, enabled })
invoke('launch_smapi')
invoke<Settings>('get_settings')
invoke('save_settings', { settings })
invoke<GamePaths | null>('discover_paths')
```

- [ ] **Step 1: Add Pinia + vue-router; create typed wrappers**

- [ ] **Step 2: Build HomeView** — TopBar (刷新 / 启动), ModTable (开关、名称、作者、版本、状态), StatusBar

- [ ] **Step 3: SettingsView** — path inputs + 浏览 (`tauri-plugin-dialog`) + 自动探测 + 保存

- [ ] **Step 4: zh-CN copy** for all visible strings in these views

- [ ] **Step 5: Manual run**

```powershell
npm run tauri dev
```

Expected: app opens; with valid paths, list populates; toggle renames folder; launch starts SMAPI if installed.

- [ ] **Step 6: Commit**

```powershell
git commit -m "feat: Vue main list and settings for local mod management"
```

---

### Task 8: Profiles domain + store

**Files:**
- Create: `src-tauri/src/domain/profiles.rs`
- Create: `src-tauri/src/storage/profiles_store.rs`
- Create: `src-tauri/src/commands/profiles.rs`
- Test: profile diff + apply against temp Mods

**Interfaces:**
- Produces:
  - `Profile { id, name, created_at, updated_at, enabled_mod_ids: Vec<String> }`
  - `fn profile_diff(current_enabled: &[String], target: &[String]) -> (Vec<String> /*to_enable*/, Vec<String> /*to_disable*/)`
  - `fn apply_profile(mods_path, profile, current_entries) -> AppResult<ApplyReport>`
  - `ApplyReport { enabled: usize, disabled: usize, errors: Vec<String> }`
  - Commands: `list_profiles`, `create_profile`, `rename_profile`, `delete_profile`, `apply_profile`, `snapshot_current_as_profile`

First run: if no profiles, create `default` from current enabled UniqueIDs.

- [ ] **Step 1: Failing test for diff**

```rust
#[test]
fn diff_enables_and_disables() {
    let (en, dis) = profile_diff(&["A".into(), "B".into()], &["B".into(), "C".into()]);
    assert_eq!(en, vec!["C".to_string()]);
    assert_eq!(dis, vec!["A".to_string()]);
}
```

- [ ] **Step 2: Implement store under `%APPDATA%/svmm/profiles/*.json` + apply; tests pass**

- [ ] **Step 3: Commit**

```powershell
git commit -m "feat: mod profiles with diff apply"
```

---

### Task 9: Profiles UI

**Files:**
- Create: `src/views/ProfilesView.vue`, `src/stores/profiles.ts`
- Modify: `TopBar.vue` (profile select), `router/index.ts`

- [ ] **Step 1: Profile dropdown on TopBar calls `apply_profile`**

- [ ] **Step 2: ProfilesView CRUD in zh-CN; confirm before delete**

- [ ] **Step 3: Manual test switch profiles; commit**

```powershell
git commit -m "feat: profiles UI and top-bar switcher"
```

---

### Task 10: SMAPI update checking

**Files:**
- Create: `src-tauri/src/domain/smapi_update.rs`
- Create: `src-tauri/src/commands/smapi.rs` (extend) or `commands/updates.rs`
- Modify: `ModEntry.status` update path; `ModTable.vue` badges
- Add: `reqwest` with `json` feature; cache `cache/update_check.json`
- Test: mock JSON parse test (no live network required in unit test)

**Interfaces:**
- Produces:
  - `fn check_updates(mods: &[ModEntry]) -> AppResult<Vec<UpdateInfo>>`
  - `UpdateInfo { id, status, suggested_version: Option<String>, error_reason: Option<String> }`
  - Command: `check_mod_updates`
  - On settings `check_updates_on_startup`, HomeView triggers check after first scan

Request shape (SMAPI v3):

```json
{ "mods": [ { "id": "Ada.TestMod", "updateKeys": ["Nexus:1234"] } ] }
```

Map response flags to `update_available` | `unofficial_update` | `broken` | `ok`.

- [ ] **Step 1: Parse fixture response in unit test**

- [ ] **Step 2: Implement HTTP client + merge into list statuses**

- [ ] **Step 3: UI badge + 「检查更新」 button; commit**

```powershell
git commit -m "feat: SMAPI mod update checking"
```

---

### Task 11: Nexus API client + secure key

**Files:**
- Create: `src-tauri/src/storage/secure_key.rs`
- Create: `src-tauri/src/domain/nexus/mod.rs`, `client.rs`
- Create: `src-tauri/src/commands/nexus.rs`
- Add deps: `keyring`, `reqwest`
- Test: client builds correct headers (API key header `apikey`)

**Interfaces:**
- Produces:
  - `fn set_nexus_api_key(key: &str) -> AppResult<()>`
  - `fn clear_nexus_api_key() -> AppResult<()>`
  - `fn has_nexus_api_key() -> bool`
  - `fn validate_nexus_api_key() -> AppResult<NexusUser>` — `GET https://api.nexusmods.com/v1/users/validate.json`
  - Commands: `nexus_set_key`, `nexus_clear_key`, `nexus_status`, `nexus_validate`

Never include key in `AppError.detail` or log lines.

- [ ] **Step 1: Tests for header construction with dummy key**

- [ ] **Step 2: Implement keyring + validate; Settings UI fields (masked)**

- [ ] **Step 3: Commit**

```powershell
git commit -m "feat: Nexus API key storage and validation"
```

---

### Task 12: Nexus download, zip install, NXM deep link

**Files:**
- Create: `src-tauri/src/domain/mods/install.rs`
- Create: `src-tauri/src/domain/nexus/download.rs`
- Modify: `tauri.conf.json` + deep-link plugin for `nxm`
- Modify: `lib.rs` to handle deep-link events
- Test: zip slip rejection tests; install happy path with fixture zip

**Interfaces:**
- Produces:
  - `fn safe_extract_zip(zip_path: &Path, dest_mods: &Path) -> AppResult<PathBuf>`
  - `fn install_mod_zip(zip_path: &Path, mods_path: &Path) -> AppResult<ModEntry>`
  - `fn handle_nxm_url(url: &str) -> AppResult<ModEntry>` — parse `nxm://stardewvalley/mods/<id>/files/<fileId>?key=...&expires=...`
  - Commands: `install_mod_zip`, `install_from_nxm`

Zip slip test:

```rust
#[test]
fn rejects_path_traversal() {
    // zip containing ../evil.dll → AppError code "unsafe_zip"
}
```

- [ ] **Step 1: Implement safe_extract + tests**

- [ ] **Step 2: Nexus download file endpoint + install**

- [ ] **Step 3: Register `nxm` protocol (Windows) via tauri-plugin-deep-link; on event call `handle_nxm_url`**

- [ ] **Step 4: UI: drag-drop zip on HomeView; toast on NXM install**

- [ ] **Step 5: Commit**

```powershell
git commit -m "feat: Nexus NXM download and safe zip install"
```

---

### Task 13: Endorse + Premium update

**Files:**
- Modify: `src-tauri/src/domain/nexus/client.rs`
- Modify: `ModTable.vue` / row actions
- Test: URL path builders for endorse + download link

**Interfaces:**
- Produces:
  - `fn endorse_mod(nexus_mod_id: u32) -> AppResult<()>` — POST endorse endpoint
  - `fn update_mod_from_nexus(mod_entry: &ModEntry) -> AppResult<ModEntry>` — resolve Nexus file, download, replace folder (backup old to `Mods/.svmm-backup-<id>-<ts>` then extract; on failure restore)
  - Commands: `nexus_endorse`, `nexus_update_mod`
  - If API indicates non-premium / 403 on quick download: return `AppError { code: "nexus_premium_required", message: "需要 Nexus Premium 才能在应用内更新" }`

Parse `Nexus:1234` from `update_keys` for IDs.

- [ ] **Step 1: Implement endorse + update with premium error mapping**

- [ ] **Step 2: UI actions 「推荐」「更新」; commit**

```powershell
git commit -m "feat: Nexus endorse and in-app mod update"
```

---

### Task 14: Logging, theme, polish, Windows package metadata

**Files:**
- Create: `src-tauri/src/storage/log_util.rs`
- Modify: Settings (打开日志目录), theme class on `document.documentElement`
- Modify: `tauri.conf.json` bundle icons/identifier
- Create: root `README.md` (install/run instructions in 中文)

**Interfaces:**
- Produces: `fn open_log_dir() -> AppResult<()>`
- Rolling or append log at `%APPDATA%/svmm/logs/svmm.log` for command errors

- [ ] **Step 1: Log helper + open folder command**

- [ ] **Step 2: Theme bind from settings; dark/light CSS variables (avoid purple-gradient AI cliché; use warm earth tones fitting Stardew — greens/soil, not generic purple)**

- [ ] **Step 3: README + `npm run tauri build` smoke on Windows**

- [ ] **Step 4: Commit**

```powershell
git commit -m "feat: logging, theme, and Windows polish"
```

---

## Spec Coverage Checklist

| Spec section | Tasks |
|--------------|-------|
| Architecture / modules | 1–2, file structure |
| Settings + app data | 2 |
| Path discovery | 3 |
| Manifest + scan + enable | 4–5 |
| Launch SMAPI | 6 |
| Vue UI main/settings | 7 |
| Profiles | 8–9 |
| Update checking | 10 |
| Nexus key / NXM / install | 11–12 |
| Endorse / Premium update | 13 |
| Errors / zip safety / logging / theme | 2, 12, 14 |
| Phased delivery | Tasks ordered 1→14 |

## Self-Review Notes

- No TBD steps; enable/disable, SMAPI update URL, and keyring service names are locked in Global Constraints.
- DTO casing locked to camelCase over IPC.
- Types `ModEntry`, `Profile`, `Settings`, `GamePaths`, `ApplyReport`, `UpdateInfo` introduced before UI/Nexus consumers.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-16-svmm-implementation.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between tasks  
2. **Inline Execution** — execute tasks in this session with executing-plans checkpoints  

Which approach?
