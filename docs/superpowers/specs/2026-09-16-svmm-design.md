# SVMM Design Spec

**Date:** 2026-09-16  
**Product:** SVMM (Stardew Valley Mod Manager)  
**Reference:** [Floogen/Stardrop](https://github.com/Floogen/Stardrop)  
**Status:** Approved for implementation planning

## 1. Goals

Build a Windows-first, Tauri-based Stardew Valley mod manager with near feature parity to Stardrop (scope tier C), including:

- Local mod scan, enable/disable, and SMAPI launch
- Mod profiles
- Update / compatibility checking via SMAPI metadata
- Nexus Mods API (API key, NXM install, endorse; Premium one-click update)

SVMM must be **non-destructive**: uninstalling SVMM must not break SMAPI, the game, or the user's Mods folder beyond reversible enable/disable state.

### Non-goals (v1)

- macOS / Linux support (architecture may allow later)
- Automatic dependency installation
- Multi-game support
- Full theme marketplace / custom theme packs
- Steam overlay launch integration (optional later)

## 2. Decisions

| Topic | Choice |
|-------|--------|
| Scope | Near-complete Stardrop parity (incl. Nexus) |
| Shell | Tauri 2 |
| Frontend | Vue 3 + TypeScript + Vite |
| Backend | Rust domain modules behind Tauri commands |
| Platform | Windows first |
| Product name | SVMM |
| UI language | `zh-CN` first; reserve `en` |
| Architecture style | Layered Tauri app (not a line-by-line Stardrop port) |

## 3. Architecture

```
[Vue UI] --IPC--> [Tauri Commands] --> [Domain Modules] --> [FS / SMAPI / Nexus API]
                                         └--> [%APPDATA%/svmm]
```

### Layers

| Layer | Responsibility |
|-------|----------------|
| Vue UI | Lists, profiles, settings, Nexus UX, progress, errors |
| Tauri IPC | Typed commands (`scan_mods`, `set_mod_enabled`, `apply_profile`, …) |
| Rust modules | `game`, `mods`, `profiles`, `smapi`, `nexus` |
| Local storage | Settings, profiles, optional caches, logs |

### Principles

1. **Single source of truth for enablement:** filesystem state under `Mods/`; profiles are target sets applied to FS.
2. **UI never mutates disk directly** — only via IPC commands.
3. **Ship in phases; design for full scope** so Nexus/profiles do not require a rewrite.

### Suggested crate / module layout

```
src-tauri/src/
  lib.rs / main.rs
  error.rs          # AppError
  commands/         # IPC handlers
  domain/
    game.rs         # path discovery & validation
    mods.rs         # scan, enable/disable, install zip
    profiles.rs
    smapi.rs        # launch, update-key checks
    nexus.rs        # API, download, NXM
  storage/
    settings.rs
    profiles_store.rs
    secure_key.rs   # API key storage
```

Frontend (Vue) keeps thin stores that call invoke wrappers; no business rules duplicated in TS beyond presentation.

## 4. Data Model

### ModEntry (runtime scan result)

| Field | Notes |
|-------|-------|
| `id` | Prefer `manifest.UniqueID`; else relative folder path |
| `name`, `author`, `version`, `description` | From `manifest.json` |
| `folder_path` | Relative path under `Mods/` |
| `enabled` | Current FS enable state |
| `minimum_api_version` | Compatibility hints |
| `update_keys` | e.g. `Nexus:1234` |
| `dependencies` | Hard deps for warnings only |
| `status` | `ok` \| `missing_manifest` \| `incompatible` \| `update_available` \| `unofficial_update` \| `broken` \| … |

### Profile

| Field | Notes |
|-------|-------|
| `id`, `name` | UUID + display name |
| `created_at`, `updated_at` | ISO timestamps |
| `enabled_mod_ids` | Set of UniqueIDs that should be enabled |

Switching a profile applies the set via batch enable/disable; it does **not** copy the Mods tree.

### Settings

| Field | Notes |
|-------|-------|
| `game_path`, `smapi_path`, `mods_path` | Auto-detect + manual override |
| `last_profile_id` | |
| `theme` | `system` \| `light` \| `dark` |
| `language` | default `zh-CN` |
| `check_updates_on_startup` | bool |
| Nexus API key | Stored via Windows secure storage / encrypted local store — **never** plain in git or logs |

### On-disk layout

```
%APPDATA%/svmm/
  settings.json
  profiles/
    default.json
    <uuid>.json
  cache/
    nexus_meta.json       # optional
    update_check.json     # optional
  logs/
    svmm.log
```

### Bootstrap

On first run, create a `default` profile snapshot from the current Mods enablement state.

## 5. Core Flows

### 5.1 Startup & path discovery

1. Load `settings.json`.
2. If paths missing, probe common Windows locations (Steam, Xbox/Game Pass, GOG, manual installs).
3. Validate `StardewModdingAPI.exe` and `Mods/`.
4. On failure, open Settings and require manual paths; do not crash the shell.

### 5.2 Scan mods

1. Recursively scan `Mods/` (including nested folders).
2. Parse `manifest.json` per package; missing manifest → `missing_manifest`, still listable/toggleable.
3. Compute dependency / API compatibility statuses.
4. Return `ModEntry[]` to the UI.

### 5.3 Enable / disable

- Follow SMAPI / Stardrop conventions (implementation must pick **one** fixed mechanism after verifying current Stardrop/SMAPI behavior — e.g. `.disabled` directory suffix or equivalent).
- Per-mod operations are atomic: on failure, roll back that entry and return an error.

### 5.4 Profile switch

1. Optionally save current enable set into the active profile before switch.
2. Load target `enabled_mod_ids`.
3. Diff against FS; batch enable/disable.
4. Refresh list; set `last_profile_id`.

### 5.5 Launch game

- Spawn `StardewModdingAPI.exe` with working directory = game root via Tauri process API.
- Steam-launch path is out of scope for v1.

### 5.6 Update checking (metadata)

1. Read each mod's `UpdateKeys`.
2. Query SMAPI-compatible update/compatibility endpoints (same class of sources Stardrop uses).
3. Mark rows: `update_available`, `unofficial_update`, `broken`, etc.

### 5.7 Nexus Mods

1. User pastes API key → validate → store securely.
2. Register Windows `nxm://` protocol handler for SVMM.
3. On NXM URL: download via Nexus API → safe extract into `Mods/` → rescan.
4. Premium: in-app update for Nexus-hosted installed mods; non-Premium: open web / explain limit.
5. Endorsement for installed mods with known Nexus IDs.

### 5.8 Local zip / folder install

- Drag-drop or file picker → require/prefer manifest → extract/copy into `Mods/` with path-traversal guards → rescan.

## 6. UI Structure

Desktop-utility density (Stardrop-like), not a marketing landing page. Chinese UI first.

### Main window

- **Top bar:** SVMM title, profile dropdown, refresh, launch SMAPI, settings
- **Toolbar:** search; filters (all / enabled / disabled; update / incompatible)
- **Table:** toggle, name, author, version, status badges, folder
- **Status bar:** mod counts, last update check, Nexus connection indicator

### Settings

- Paths (browse + auto-detect)
- Theme, language, check-updates-on-startup
- Nexus API key (masked, clearable)

### Profiles

- Create / rename / delete / apply (page or modal)

### Routes (minimal)

- `/` — main list  
- `/settings` — settings  
- `/profiles` — profile management (optional modal instead)

### UX rules

- Destructive actions need confirmation (delete profile, overwrite same-folder install).
- Long tasks show progress; downloads are cancellable.
- Themes: `light` / `dark` / `system` only in v1.

## 7. Errors, Security, Logging

### Errors

- Unified serializable `AppError`: `code`, user-facing `message`, optional `detail` for logs.
- UI surfaces path errors, manifest parse failures, Nexus 401/rate limits, permission issues, download verification failures.
- Batch profile apply: report partial success/failure; keep state recoverable.
- If files are locked by a running game, tell the user to quit the game first.

### Security

- Nexus API key never in repo or plaintext logs.
- Zip extract must reject `..` / absolute paths escaping `Mods/`.
- `nxm://` handler only accepts Stardew Valley Nexus links.

### Logging

- `%APPDATA%/svmm/logs/svmm.log`
- Settings action: “Open log folder”

## 8. Testing Strategy

| Level | Coverage |
|-------|----------|
| Rust unit | Manifest parse, profile diff, path normalize, safe zip extract |
| Rust integration | Temp Mods tree: scan / toggle / apply profile |
| Frontend | Light tests for filters / error display; no heavy E2E in v1 |
| Manual checklist | Missing SMAPI, bad manifest, bad Nexus key, NXM install, profile switch then launch |

## 9. Delivery Phases

All phases share the architecture above.

1. **Scaffold + local core** — Tauri/Vue project, path discovery, scan, enable/disable, launch SMAPI  
2. **Profiles** — CRUD + apply + default snapshot  
3. **Update checking** — UpdateKeys + status badges  
4. **Nexus** — API key, download/install, NXM protocol, endorse, Premium update path  
5. **Polish** — theme, logging UX, Windows installer packaging  

## 10. Open Implementation Notes

These are fixed during implementation planning / first coding spike, not left as product ambiguity:

1. Exact enable/disable filesystem convention (verify against current SMAPI + Stardrop).
2. Exact SMAPI update/compatibility HTTP endpoints and response mapping.
3. Concrete Windows API for storing the Nexus key (Credential Manager vs encrypted file).
4. Tauri 2 plugin choices for dialog, process, deep-link/`nxm` registration.

## 11. Success Criteria

- User can point SVMM at a valid SMAPI install, see all mods, toggle them, manage profiles, and launch the game.
- Update badges appear for mods with valid UpdateKeys when the network check succeeds.
- With a valid Nexus key, user can install via NXM and endorse; Premium users can update in-app.
- No plaintext API keys in logs; zip installs cannot write outside `Mods/`.
- App remains usable when paths are wrong (guided settings, not a hard crash).
