# Phase 5A: Settings Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the app-settings store and the behaviors that depend on it, and surface mutation errors in the UI. Introduce a `settings.rs` module in `pond-tauri` (a `Settings` struct + `load`/`save` to `settings.json` in the OS **config dir**, atomic temp+rename), with `get_settings`/`set_settings` IPC and a `store_path` helper for System Info. On the frontend: settings state in `App`, an `updateSettings(patch)` helper, **auto-draft** on title edits (a pure tested helper applied in `InlineEditor`), an **always-on-top** window toggle, footer-menu items (Automatic Drafts / Always On Top / Settings…), a **Settings dialog** shell with the **System Information** tab, and **error dialogs** (command rejections shown in an `AlertDialog` instead of `console.error`). **No `pond-core` changes** — settings are GUI-only and live in `pond-tauri`.

**Architecture:** `pond-core` stays the single source of truth for tasks; **settings are a separate `pond-tauri`-local concern**. A `Settings` value is held in Tauri state behind a `Mutex` (`app.manage(Mutex::new(settings))`), loaded once at startup from `<config_dir>/pond/settings.json`. `get_settings` reads the Mutex; `set_settings` replaces the in-memory value, persists it to disk (atomic temp+rename, `serde_json` pretty), and returns the persisted value (whole-object commands — the frontend reads, mutates a field, writes the whole object back). The existing managed `TaskStore` is untouched; a tiny `store_path` command exposes `store.file_path()` for System Info. The frontend keeps its one `invoke` site (`api/client.ts`), holds settings in `App` state with an `updateSettings(patch)` merge helper, threads `usesAutoDraft` down to the `InlineEditor`, and applies `alwaysOnTop` via `@tauri-apps/api/window`. The auto-draft status decision is a **pure, unit-tested helper** (`state/autodraft.ts`); the editor's title save/commit sites apply it. Errors are surfaced via an `errorMessage` + `onError(msg)` callback threaded alongside `onSnapshot`, rendered in a Radix `AlertDialog` host.

**Tech Stack:** Rust 1.96.0 (pinned), Tauri v2, `serde`/`serde_json`, `directories` 5 (added to `pond-tauri`, matching `pond-core`), `tempfile` (dev); Vite + React 18 + TypeScript + `@radix-ui/themes` 3.3.0 + `@radix-ui/react-icons`; `@tauri-apps/api` v2 (`core`, `event`, `window`, `app`); Vitest. npm.

---

## Conventions (read this section before `## File Structure`)

Every task obeys these. They are not repeated per step.

- **Branch:** work on the existing `tauri-radix-migration` branch. Do **not** create a new branch and do **not** set an upstream.
- **Rust toolchain:** pinned `1.96.0` (already in `rust-toolchain.toml`). Run all `cargo`/`npm`/`npx` commands from the repo root (the Vite root is the repo root).
- **Per Rust task gate:** `cargo fmt --all` then `cargo clippy --workspace --all-targets -- -D warnings` must be clean, and `cargo test -p pond-tauri` green.
- **Per frontend task gate:** `npx tsc --noEmit` clean, `npm run build` succeeds, `npx vitest run` green.
- **Imports/`use` at the top:** ALL `import` (TS) and `use` (Rust) statements live at the top of the file. In Rust test modules, all `use` go at the top of `mod tests` (i.e. directly under `#[cfg(test)] mod tests {`). No mid-file imports.
- **Radix Themes defaults only:** stock Radix parts, built-in named palette, no theme customization. The TSX below targets the installed `@radix-ui/themes` **3.3.0** API. Where a sample's component/prop shape differs from what 3.3.0 actually exports (e.g. `Tabs`, `Dialog`, `AlertDialog`, `DropdownMenu.CheckboxItem`), **adjust the usage to the installed API** — the gate is a clean `tsc --noEmit` + `npm run build` (report any adjustment in the task's commit/notes). This is the established Phase 3–4 latitude, **not** a placeholder to leave logic unwritten.
- **No visual / DOM / screenshot tests.** Verification is logic unit tests (the `autoDraftStatus` Vitest suite, the `settings.rs` Rust tests, the `client.test.ts` mocked-invoke assertions) + the build/typecheck gates + manual `cargo tauri dev` launch (human visual check). Do **not** add `@testing-library`/`jsdom` render tests for the dialog, footer, window toggle, or error host.
- **Command (invoke) names** must equal exactly what the frontend `client.ts` wrapper passes to `invoke` (`get_settings`, `set_settings`, `store_path`).
- **Commit trailer:** every commit message ends with a trailing line:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`

### Verified facts (source of truth — do not guess)

- **`directories` is NOT a `pond-tauri` dependency yet.** It is a `pond-core` dep (`directories = "5"`, see `crates/pond-core/Cargo.toml`). Task 1 **adds** `directories = "5"` to `src-tauri/Cargo.toml` so `settings.rs` can resolve the config dir directly.
- **App identifiers:** `pond-core` builds its data dir with `ProjectDirs::from("", "", "pond")` and uses `.data_dir()` (`crates/pond-core/src/paths.rs`). `settings.rs` uses the **same triple** `ProjectDirs::from("", "", "pond")` but `.config_dir()` (macOS resolves both to `~/Library/Application Support/pond`; Linux config = `~/.config/pond`, data = `~/.local/share/pond`; Windows config = `%APPDATA%\pond` roaming). The fallback when `ProjectDirs` is `None` is `PathBuf::from("pond")` (mirrors `data_directory`).
- **Store path for System Info:** `TaskStore` exposes `file_path() -> &Path` (used by the Phase 1–3 shell); `store_path` returns `store.file_path().display().to_string()`. This already honors the `POND_STORE` override because `TaskStore::open_default()` was constructed from `pond_core::paths::default_store_path()`.
- **`get_version`** is exported from `@tauri-apps/api/app` (`import { getVersion } from "@tauri-apps/api/app"`). **`getCurrentWindow().setAlwaysOnTop(bool)`** is from `@tauri-apps/api/window`.
- **Always-on-top capability:** the Tauri v2 permission identifier is **`core:window:allow-set-always-on-top`** (the per-command allow permission under the `core:window` set). `capabilities/default.json` currently grants only `core:default`, which does **not** include `set_always_on_top`. Add the explicit permission (Task 5). If `tsc`/runtime reports the identifier differs in the installed Tauri, verify against `src-tauri/gen/schemas/desktop-schema.json` and adjust; gate = clean build + the call not rejected at manual launch.
- **Swift parity (source of truth):**
  - `AppModel.usesAutoDraft` defaults `true` (`UserDefaults … as? Bool ?? true`); `autoDraftEditStatus = usesAutoDraft ? .draft : nil`; `autoDraftConfirmationStatus = usesAutoDraft ? .ready : nil` (`Sources/PondApp/AppModel.swift`).
  - `DetailView.saveTitle`: `statusAfterEdit = title == item.title ? nil : model.autoDraftEditStatus`. `DetailView.confirmationStatus(for:) = item.status == .draft ? .ready : model.autoDraftConfirmationStatus` (`Sources/PondApp/DetailView.swift`). These map exactly to the `autodraft.ts` pseudocode below.
  - `SidebarView` footer toggle order: `Toggle("Automatic Drafts", isOn: $model.usesAutoDraft)`, `Toggle("Always On Top", isOn: $alwaysOnTop)`, then `Button("Settings…") { openSettings() }`. (Hide Completed / Show Archived / Add a Group already exist in the Tauri footer.)
  - `SettingsView.systemInformationSection`: `LabeledContent("Version")` from `CFBundleShortVersionString`, `LabeledContent("Build")` from `CFBundleVersion` (`Sources/PondApp/SettingsView.swift`).
  - `AppModel.errorMessage: String?` set from `error.localizedDescription` at each mutation catch site — the parity for the error dialog.

### Divergences from the design spec (confirmed against source)

1. **`store_path` command (not in the spec's command list).** The spec's System Information tab needs the store path; this plan adds a tiny `store_path(store: State<TaskStore>) -> String` command rather than re-deriving the path on the frontend. This keeps `POND_STORE` resolution server-side and avoids a second `directories` call in JS.
2. **`directories` added to `pond-tauri`.** The spec says "via the `directories` crate (same one `pond-core` uses)"; since it is not yet a `pond-tauri` dep, Task 1 adds `directories = "5"` to `src-tauri/Cargo.toml`.
3. **Settings commands return the whole `Settings` object** (`get_settings`/`set_settings` both `-> Result<Settings, String>`), per confirmed decision 2 — not typed per-field setters.

---

## File Structure

```
src-tauri/
├─ Cargo.toml      + directories = "5" dependency
└─ src/
   ├─ settings.rs  (NEW) Settings struct (serde camelCase, #[serde(default)]) + settings_path()/load()/save() (atomic temp+rename, serde_json pretty) + #[cfg(test)] tempdir tests
   ├─ commands.rs  + get_settings / set_settings / store_path #[tauri::command] wrappers
   └─ main.rs      + mod settings; load settings at startup; app.manage(Mutex<Settings>); register the 3 commands

src/
├─ api/
│  ├─ client.ts       + getSettings() / setSettings(s) / storePath() wrappers
│  ├─ types.ts        + Settings interface (mirrors the DTO, camelCase)
│  └─ client.test.ts  + mocked-invoke assertions for getSettings/setSettings
├─ state/
│  ├─ autodraft.ts      (NEW) pure autoDraftStatus({ usesAutoDraft, currentStatus, phase, titleChanged })
│  └─ autodraft.test.ts (NEW) every branch (edit/confirm × draft/non-draft × on/off × titleChanged)
├─ components/
│  ├─ SettingsDialog.tsx (NEW) Radix Dialog + Tabs shell; System Information tab (Version/Build via getVersion + store path via storePath())
│  ├─ Sidebar.tsx        footer: + Automatic Drafts / Always On Top CheckboxItems + Settings… item (opens the dialog)
│  ├─ DetailPane.tsx     thread usesAutoDraft + onError down to TaskRow
│  ├─ TaskRow.tsx        thread usesAutoDraft + onError down to InlineEditor; .catch → onError
│  └─ InlineEditor.tsx   apply auto-draft status on title edit/confirm (reads usesAutoDraft); .catch → onError
└─ App.tsx        settings state (fetch on mount) + updateSettings(patch); apply always-on-top; restore/persist lastSelectedCollection; errorMessage + onError + AlertDialog error host; mount the Settings dialog
```

Each unit stays focused: `settings.rs` is the only place that touches `settings.json` (tested with a tempdir + injected path); `commands.rs` wrappers contain no logic beyond Mutex access → string error; `api/` is the only `invoke` site; `state/autodraft.ts` is pure (tested); components render and dispatch.

---

## Task 1: `settings.rs` module + `get_settings` / `set_settings` / `store_path` commands + startup wiring

Establishes the settings store: a `Settings` DTO (serde camelCase, `#[serde(default)]` so partial/absent files load with defaults), a config-dir path, atomic load/save, Mutex state, and three command wrappers. Tests run against a tempdir with an **injected** path (no global state).

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Add the `directories` dependency**

Edit `src-tauri/Cargo.toml`. In `[dependencies]`, after `notify = "6"`, add:

```toml
directories = "5"
```

(Matches `pond-core`'s pin so the workspace resolves a single version.)

- [ ] **Step 2: Write `settings.rs` with failing tests first**

Create `src-tauri/src/settings.rs`. Write the full module — `Settings`, `settings_path`, `load`, `save`, and the test module. The tests reference `load`/`save` with an injected path, so they compile against the implementation in the same step (red comes from running before `mod settings;` is declared in Step 4; if you prefer red-via-assert, declare `mod settings;` first, then watch the round-trip assertion fail before `save` is written — either order reaches green by Step 5).

```rust
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// App-settings, persisted to `<config_dir>/pond/settings.json`. GUI-only
/// (`pond-tauri`-local); the CLI does not read these. `#[serde(default)]` on
/// every field means a partial or absent file loads with defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Editing a title drops the task to Draft; confirming promotes to Ready.
    pub uses_auto_draft: bool,
    /// Keep the window above other windows.
    pub always_on_top: bool,
    /// Default prompt template for new collections (used in 5B).
    pub default_prompt_template: String,
    /// The collection selected when the app was last closed (restored on launch).
    pub last_selected_collection: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            uses_auto_draft: true,
            always_on_top: false,
            default_prompt_template: String::new(),
            last_selected_collection: None,
        }
    }
}

/// `<config_dir>/pond/settings.json`. Same app identifiers as
/// `pond_core::paths` (`ProjectDirs::from("", "", "pond")`), but the config dir
/// rather than the data dir. Falls back to a relative `pond/settings.json`.
pub fn settings_path() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("", "", "pond") {
        return dirs.config_dir().join("settings.json");
    }
    PathBuf::from("pond").join("settings.json")
}

/// Load settings from `path`. A missing or corrupt file yields `Settings::default()`
/// (a fresh, valid file is written on the next `save`).
pub fn load(path: &Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Persist settings to `path` (pretty JSON, atomic temp+rename). Creates the
/// parent directory if needed.
pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json.as_bytes())?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn defaults_when_file_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = load(&path);
        assert_eq!(s, Settings::default());
        assert!(s.uses_auto_draft);
        assert!(!s.always_on_top);
        assert_eq!(s.default_prompt_template, "");
        assert_eq!(s.last_selected_collection, None);
    }

    #[test]
    fn partial_file_fills_missing_fields_with_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        // Only one field present; the rest must default.
        std::fs::write(&path, br#"{ "alwaysOnTop": true }"#).unwrap();
        let s = load(&path);
        assert!(s.always_on_top);
        assert!(s.uses_auto_draft); // default true survives
        assert_eq!(s.last_selected_collection, None);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, b"not json at all").unwrap();
        assert_eq!(load(&path), Settings::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let original = Settings {
            uses_auto_draft: false,
            always_on_top: true,
            default_prompt_template: "Plan: {{title}}".to_string(),
            last_selected_collection: Some("Work/Docs".to_string()),
        };
        save(&path, &original).unwrap();
        assert_eq!(load(&path), original);
    }

    #[test]
    fn save_overwrites_persisted_value() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        save(&path, &Settings::default()).unwrap();
        let updated = Settings {
            last_selected_collection: Some("Inbox".to_string()),
            ..Settings::default()
        };
        save(&path, &updated).unwrap();
        let loaded = load(&path);
        assert_eq!(loaded.last_selected_collection, Some("Inbox".to_string()));
        // No leftover temp file.
        assert!(!path.with_extension("json.tmp").exists());
    }
}
```

Run (after Step 4 declares the module, or expect a "module not found" red now):

```bash
cargo test -p pond-tauri settings
```
Expected (once `mod settings;` exists): all 5 `settings::tests` pass.

- [ ] **Step 3: Add the three command wrappers to `commands.rs`**

Edit `src-tauri/src/commands.rs`. Extend the top `use` block to bring in the settings type and `Mutex`, and append the three commands before `#[cfg(test)]`. The top imports become:

```rust
use crate::dto::{CollectionGroupSummaryDto, CollectionSummaryDto, SnapshotDto};
use crate::mutations;
use crate::settings::{self, Settings};
use pond_core::{CollectionColor, Result, TaskItem, TaskStatus, TaskStore};
use std::sync::Mutex;
use tauri::State;
```

Append (before the `#[cfg(test)]` test module):

```rust
#[tauri::command]
pub fn get_settings(state: State<Mutex<Settings>>) -> std::result::Result<Settings, String> {
    let guard = state.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn set_settings(
    state: State<Mutex<Settings>>,
    settings: Settings,
) -> std::result::Result<Settings, String> {
    settings::save(&settings::settings_path(), &settings).map_err(|e| e.to_string())?;
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    *guard = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub fn store_path(store: State<TaskStore>) -> String {
    store.file_path().display().to_string()
}
```

Note: `set_settings` writes to disk first, then updates the in-memory Mutex, so a failed write does not desync state from the file. `Settings` deserializes from the camelCase JS object the frontend sends.

- [ ] **Step 4: Declare the module, load settings at startup, manage the Mutex, register commands in `main.rs`**

Edit `src-tauri/src/main.rs`:
1. Add `mod settings;` to the module list (after `mod mutations;`).
2. In `.setup(...)`, after `app.manage(pond_core::TaskStore::open_default());`, load and manage the settings Mutex.
3. Add the three commands to `generate_handler!`.

The module list becomes:

```rust
mod commands;
mod dto;
mod mutations;
mod settings;
mod watcher;
```

In `generate_handler!`, append after `commands::delete_group,`:

```rust
            commands::get_settings,
            commands::set_settings,
            commands::store_path,
```

In `.setup(|app| { ... })`, immediately after `app.manage(pond_core::TaskStore::open_default());`, add:

```rust
            let loaded_settings = settings::load(&settings::settings_path());
            app.manage(std::sync::Mutex::new(loaded_settings));
```

- [ ] **Step 5: Test + gate**

```bash
cargo test -p pond-tauri
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: `settings::tests` (5) + existing `commands`/`mutations`/`dto`/`watcher` tests pass; fmt/clippy clean.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/settings.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(tauri): settings store + get/set_settings + store_path commands

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 2: Frontend settings client + types + `App` settings state + `lastSelectedCollection`

Add the `Settings` interface, three client wrappers, mocked-invoke tests, and wire `App` to fetch settings on mount, expose `updateSettings(patch)`, and restore/persist `lastSelectedCollection`.

**Files:**
- Modify: `src/api/types.ts`, `src/api/client.ts`, `src/api/client.test.ts`, `src/App.tsx`

- [ ] **Step 1: Add a failing test for the two settings wrappers**

Append to `src/api/client.test.ts` (inside the existing `describe`):

```ts
  it("getSettings invokes get_settings with no args", async () => {
    const settings = {
      usesAutoDraft: true,
      alwaysOnTop: false,
      defaultPromptTemplate: "",
      lastSelectedCollection: null,
    };
    invokeMock.mockResolvedValue(settings);
    await expect(getSettings()).resolves.toEqual(settings);
    expect(invokeMock).toHaveBeenCalledWith("get_settings");
  });

  it("setSettings invokes set_settings with the whole settings object", async () => {
    const settings = {
      usesAutoDraft: false,
      alwaysOnTop: true,
      defaultPromptTemplate: "",
      lastSelectedCollection: "Work/Docs",
    };
    invokeMock.mockResolvedValue(settings);
    await setSettings(settings);
    expect(invokeMock).toHaveBeenCalledWith("set_settings", { settings });
  });
```

Extend the top import in `client.test.ts` to include the new wrappers, e.g.:
`import { getSnapshot, onStoreChanged, createItem, setStatus, getSettings, setSettings } from "./client";`

Run:

```bash
npx vitest run src/api/client.test.ts
```
Expected: **fail** (`getSettings`/`setSettings` not exported).

- [ ] **Step 2: Add the `Settings` interface to `types.ts`**

Append to `src/api/types.ts`:

```ts
export interface Settings {
  usesAutoDraft: boolean;
  alwaysOnTop: boolean;
  defaultPromptTemplate: string;
  lastSelectedCollection: string | null;
}
```

- [ ] **Step 3: Add the three wrappers to `client.ts`**

Extend the top `import type` line to include `Settings`:
`import type { CollectionColor, Settings, Snapshot, TaskItem, TaskStatus } from "./types";`

Append a new section to `src/api/client.ts`:

```ts
// --- Settings ---
export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function setSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("set_settings", { settings });
}

export function storePath(): Promise<string> {
  return invoke<string>("store_path");
}
```

- [ ] **Step 4: Run the test (passing) + frontend gate**

```bash
npx vitest run src/api/client.test.ts
npx tsc --noEmit && npm run build
```
Expected: green/clean.

- [ ] **Step 5: Wire `App` — fetch settings, `updateSettings(patch)`, restore/persist `lastSelectedCollection`**

Edit `src/App.tsx`. Add the imports, a `settings` state, an `updateSettings` merge helper, fetch on mount (restoring `lastSelectedCollection` only if it still exists in the snapshot), and persist the selection when it changes.

Extend the existing imports:

```tsx
import type { Settings, Snapshot } from "./api/types";
import {
  createItem,
  deleteItem,
  getSettings,
  getSnapshot,
  onStoreChanged,
  setSettings,
} from "./api/client";
```

Add a default settings constant next to `EMPTY`:

```tsx
const DEFAULT_SETTINGS: Settings = {
  usesAutoDraft: true,
  alwaysOnTop: false,
  defaultPromptTemplate: "",
  lastSelectedCollection: null,
};
```

Inside `App`, add state and a ref (next to the existing state):

```tsx
  const [settings, setSettingsState] = useState<Settings>(DEFAULT_SETTINGS);
  const settingsRef = useRef(settings);
  settingsRef.current = settings;
```

Add the `updateSettings` merge helper (next to `apply`/`requestConfirm`):

```tsx
  // Merge a partial change into settings, persist the whole object, and update state
  // from the persisted result.
  const updateSettings = useCallback((patch: Partial<Settings>) => {
    const next = { ...settingsRef.current, ...patch };
    setSettingsState(next); // optimistic
    setSettings(next)
      .then(setSettingsState)
      .catch((e) => console.error(e));
  }, []);
```

Add a settings-fetch effect that also restores the last selection (place after the existing snapshot effect):

```tsx
  // Fetch settings on mount; restore lastSelectedCollection if it still exists.
  useEffect(() => {
    getSettings()
      .then((s) => {
        setSettingsState(s);
        const last = s.lastSelectedCollection;
        if (last) {
          const exists = snapRef.current.collections.some((c) => c.name === last);
          if (exists) setView((v) => ({ ...v, selected: last }));
        }
      })
      .catch((e) => console.error(e));
  }, []);
```

Persist the selection when it changes (skip the `ALL_COLLECTION` sentinel and avoid a redundant write):

```tsx
  // Persist the selected collection so it can be restored next launch.
  useEffect(() => {
    const sel = view.selected === ALL_COLLECTION ? null : view.selected;
    if (sel !== settingsRef.current.lastSelectedCollection) {
      updateSettings({ lastSelectedCollection: sel });
    }
  }, [view.selected, updateSettings]);
```

(Note: the restore effect reads `snapRef.current`, which is populated by the snapshot effect's `refresh()`. If the settings fetch resolves before the first snapshot, the `exists` check fails and the selection stays `ALL_COLLECTION` — acceptable; the user's last collection simply isn't pre-selected on a cold race. This matches the spec's "only if it still exists in the snapshot" guard.)

- [ ] **Step 6: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green. (No new props are passed to children yet, so the existing component signatures still typecheck.)

- [ ] **Step 7: Commit**

```bash
git add src/api/types.ts src/api/client.ts src/api/client.test.ts src/App.tsx
git commit -m "feat(ui): settings client + App settings state + lastSelectedCollection

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 3: `autodraft.ts` pure helper + tests

A pure status-decision helper mirroring `DetailView.saveTitle`/`confirmTitle`. No Tauri/React imports.

**Files:**
- Create: `src/state/autodraft.ts`, `src/state/autodraft.test.ts`

- [ ] **Step 1: Failing test — every branch**

Create `src/state/autodraft.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { autoDraftStatus } from "./autodraft";

describe("autoDraftStatus — confirm phase", () => {
  it("a draft always promotes to ready on confirm (even with auto-draft off)", () => {
    expect(
      autoDraftStatus({ usesAutoDraft: false, currentStatus: "draft", phase: "confirm", titleChanged: false }),
    ).toBe("ready");
    expect(
      autoDraftStatus({ usesAutoDraft: true, currentStatus: "draft", phase: "confirm", titleChanged: true }),
    ).toBe("ready");
  });

  it("a non-draft promotes to ready on confirm only when auto-draft is on", () => {
    expect(
      autoDraftStatus({ usesAutoDraft: true, currentStatus: "on-hold", phase: "confirm", titleChanged: true }),
    ).toBe("ready");
    expect(
      autoDraftStatus({ usesAutoDraft: false, currentStatus: "on-hold", phase: "confirm", titleChanged: true }),
    ).toBeUndefined();
  });
});

describe("autoDraftStatus — edit phase", () => {
  it("no title change → no status change", () => {
    expect(
      autoDraftStatus({ usesAutoDraft: true, currentStatus: "ready", phase: "edit", titleChanged: false }),
    ).toBeUndefined();
  });

  it("a draft stays draft while editing (no status change)", () => {
    expect(
      autoDraftStatus({ usesAutoDraft: true, currentStatus: "draft", phase: "edit", titleChanged: true }),
    ).toBeUndefined();
  });

  it("a non-draft drops to draft on edit only when auto-draft is on", () => {
    expect(
      autoDraftStatus({ usesAutoDraft: true, currentStatus: "ready", phase: "edit", titleChanged: true }),
    ).toBe("draft");
    expect(
      autoDraftStatus({ usesAutoDraft: false, currentStatus: "ready", phase: "edit", titleChanged: true }),
    ).toBeUndefined();
  });
});
```

Run:

```bash
npx vitest run src/state/autodraft.test.ts
```
Expected: **fail** (`./autodraft` missing).

- [ ] **Step 2: Implement `src/state/autodraft.ts`**

```ts
import type { TaskStatus } from "../api/types";

export type AutoDraftPhase = "edit" | "confirm";

export interface AutoDraftInput {
  /** The `usesAutoDraft` setting. */
  usesAutoDraft: boolean;
  /** The task's current status (from the snapshot item being edited). */
  currentStatus: TaskStatus;
  /** "edit" = debounced autosave / blur; "confirm" = Cmd+Enter / Tab commit. */
  phase: AutoDraftPhase;
  /** Whether the title actually changed (trimmed) vs the stored value. */
  titleChanged: boolean;
}

/**
 * The status to apply when saving a TITLE edit (notes never auto-draft).
 * `undefined` means "leave the status unchanged" (send title only).
 *
 * Mirrors Swift `DetailView` (DetailView.swift):
 *   saveTitle:    statusAfterEdit = title == item.title ? nil : autoDraftEditStatus
 *                 autoDraftEditStatus = usesAutoDraft ? .draft : nil
 *   confirmTitle: confirmationStatus = item.status == .draft ? .ready : autoDraftConfirmationStatus
 *                 autoDraftConfirmationStatus = usesAutoDraft ? .ready : nil
 */
export function autoDraftStatus({
  usesAutoDraft,
  currentStatus,
  phase,
  titleChanged,
}: AutoDraftInput): TaskStatus | undefined {
  if (phase === "confirm") {
    if (currentStatus === "draft") return "ready"; // draft → ready on confirm, always
    return usesAutoDraft ? "ready" : undefined; // non-draft → ready when auto-draft on
  }
  // phase === "edit"
  if (!titleChanged) return undefined; // no change → no status change
  if (currentStatus === "draft") return undefined; // a draft stays draft while editing
  return usesAutoDraft ? "draft" : undefined; // non-draft → draft when auto-draft on
}
```

- [ ] **Step 3: Run the test (passing) + gate**

```bash
npx vitest run src/state/autodraft.test.ts
npx tsc --noEmit && npm run build
```
Expected: green/clean.

- [ ] **Step 4: Commit**

```bash
git add src/state/autodraft.ts src/state/autodraft.test.ts
git commit -m "feat(ui): pure autoDraftStatus helper + tests

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 4: Wire auto-draft into `InlineEditor` (thread `usesAutoDraft` App→DetailPane→TaskRow→InlineEditor)

Thread `usesAutoDraft` down to the editor and apply `autoDraftStatus` at the two title save sites: the **edit** path (debounced autosave + blur, via `save`) and the **confirm** path (the `Commit` intent). Notes are unchanged. Gate = clean tsc+build (no DOM test — the logic is already covered by Task 3's unit suite).

**Files:**
- Modify: `src/App.tsx`, `src/components/DetailPane.tsx`, `src/components/TaskRow.tsx`, `src/components/InlineEditor.tsx`

- [ ] **Step 1: Thread the prop App → DetailPane**

Edit `src/App.tsx`. Pass `usesAutoDraft` to `DetailPane`:

```tsx
      <DetailPane
        snapshot={snapshot}
        view={view}
        focusedId={focusedId}
        editingTarget={editingTarget}
        usesAutoDraft={settings.usesAutoDraft}
        onSearch={(q) => setView((v) => ({ ...v, search: q }))}
        onFocusItem={setFocusedId}
        onEdit={onEdit}
        onEndEdit={onEndEdit}
        onSnapshot={apply}
        onRequestConfirm={requestConfirm}
      />
```

- [ ] **Step 2: DetailPane → TaskRow**

Edit `src/components/DetailPane.tsx`. Add `usesAutoDraft: boolean;` to `DetailPaneProps`, destructure it, and pass it to each `TaskRow`:

```tsx
export interface DetailPaneProps {
  snapshot: Snapshot;
  view: ViewState;
  focusedId: string | null;
  editingTarget: { id: string; field: "title" | "note" } | null;
  usesAutoDraft: boolean;
  onSearch: (q: string) => void;
  onFocusItem: (id: string | null) => void;
  onEdit: (id: string, field: "title" | "note") => void;
  onEndEdit: () => void;
  onSnapshot: (snap: Snapshot) => void;
  onRequestConfirm: (req: ConfirmRequest) => void;
}

export function DetailPane({
  snapshot, view, focusedId, editingTarget, usesAutoDraft,
  onSearch, onFocusItem, onEdit, onEndEdit, onSnapshot,
}: DetailPaneProps) {
```

In the `items.map(...)` body, add the prop to `<TaskRow … usesAutoDraft={usesAutoDraft} … />` (alongside the existing props).

- [ ] **Step 3: TaskRow → InlineEditor**

Edit `src/components/TaskRow.tsx`. Add `usesAutoDraft: boolean;` to `TaskRowProps`, destructure it, and pass it to both `InlineEditor` instances (the title one needs it; pass it to the note one too for a uniform signature — the editor ignores it for notes):

```tsx
export interface TaskRowProps {
  item: TaskItem;
  previous?: TaskItem;
  showCollection: boolean;
  collections: CollectionSummary[];
  focused: boolean;
  editingField: "title" | "note" | null;
  usesAutoDraft: boolean;
  onFocus: () => void;
  onEditTitle: () => void;
  onEditNote: () => void;
  onEndEdit: () => void;
  onMoveFocus: (dir: FocusDir) => void;
  onSnapshot: (snap: Snapshot) => void;
}

export function TaskRow({
  item, previous, showCollection, collections,
  focused, editingField, usesAutoDraft, onFocus, onEditTitle, onEditNote, onEndEdit, onMoveFocus, onSnapshot,
}: TaskRowProps) {
```

On the title `<InlineEditor field="title" …>` add `usesAutoDraft={usesAutoDraft}`; on the note `<InlineEditor field="note" …>` add `usesAutoDraft={usesAutoDraft}` as well.

- [ ] **Step 4: Apply `autoDraftStatus` in `InlineEditor`**

Edit `src/components/InlineEditor.tsx`. Add the import and the prop, then compute the status at the two title save sites.

Extend the imports:

```tsx
import { reduceKey, type EditorIntent, type FocusDir } from "../state/editor";
import { autoDraftStatus } from "../state/autodraft";
```

Add `usesAutoDraft: boolean;` to `InlineEditorProps` and destructure it:

```tsx
export interface InlineEditorProps {
  item: TaskItem;
  field: "title" | "note";
  previous?: TaskItem;
  editing: boolean;
  usesAutoDraft: boolean;
  onBeginEdit: () => void;
  onEndEdit: () => void;
  onMoveFocus: (dir: FocusDir) => void;
  onSnapshot: (snap: Snapshot) => void;
}

export function InlineEditor({
  item,
  field,
  previous,
  editing,
  usesAutoDraft,
  onBeginEdit,
  onEndEdit,
  onMoveFocus,
  onSnapshot,
}: InlineEditorProps) {
```

In `save`, the title branch currently always sends title only. Replace it so the **edit-phase** status is computed and included when defined. Replace:

```tsx
      if (field === "title") {
        if (trimmed === item.title) return; // unchanged
        const snap = await updateItem(item.id, { title: value }, item);
        onSnapshot(snap);
      } else {
```

with:

```tsx
      if (field === "title") {
        if (trimmed === item.title) return; // unchanged
        // Edit phase = debounced autosave / blur. Auto-draft may drop the task to draft.
        const status = autoDraftStatus({
          usesAutoDraft,
          currentStatus: item.status,
          phase: "edit",
          titleChanged: true, // we already returned above when unchanged
        });
        const fields = status ? { title: value, status } : { title: value };
        const snap = await updateItem(item.id, fields, item);
        onSnapshot(snap);
      } else {
```

The `Commit` intent in `execute` calls `save(draft)`, which now applies the edit-phase status — but a **confirm** (Cmd+Enter / Tab) must use the confirm phase (draft→ready). Replace the `case "Commit":` block:

```tsx
      case "Commit": {
        await save(draft);
        if (intent.thenFocus) onMoveFocus(intent.thenFocus);
        break;
      }
```

with a confirm-phase title path (notes still use plain `save`):

```tsx
      case "Commit": {
        if (field === "title") {
          clearTimer();
          const trimmed = draft.trim();
          const status = autoDraftStatus({
            usesAutoDraft,
            currentStatus: item.status,
            phase: "confirm",
            titleChanged: trimmed !== item.title,
          });
          try {
            // On confirm, always persist (title may be unchanged but status may still
            // promote a draft to ready). Send status when defined, else title only.
            const fields = status ? { title: draft, status } : { title: draft };
            onSnapshot(await updateItem(item.id, fields, item));
          } catch (e) {
            console.error(e);
          } finally {
            onEndEdit();
          }
        } else {
          await save(draft);
        }
        if (intent.thenFocus) onMoveFocus(intent.thenFocus);
        break;
      }
```

(Parity: Swift `confirmTitle` persists with `confirmationStatus(for:) ?? .draft`; here a draft always resolves to `ready` and the store keeps the title even when the trimmed text equals the stored value, so confirming an unedited draft still promotes it — matching the spec's "confirming promotes it to ready" for a fresh Cmd+N draft. The `.catch → onError` swap lands in Task 8.)

- [ ] **Step 5: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green. (Report any Radix prop-shape adjustment; none expected here — this task touches only logic and props.)

- [ ] **Step 6: Commit**

```bash
git add src/App.tsx src/components/DetailPane.tsx src/components/TaskRow.tsx src/components/InlineEditor.tsx
git commit -m "feat(ui): apply auto-draft status on title edit/confirm

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 5: Always-on-top window toggle

Add the window permission to the capability file, and apply `alwaysOnTop` via the window API on mount and whenever it changes. Gate = tsc+build (the window call is verified at manual launch).

**Files:**
- Modify: `src-tauri/capabilities/default.json`, `src/App.tsx`

- [ ] **Step 1: Grant the `set_always_on_top` permission**

Edit `src-tauri/capabilities/default.json`. Add the explicit window permission to the `permissions` array:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capability for the main window",
  "windows": ["main"],
  "permissions": ["core:default", "core:window:allow-set-always-on-top"]
}
```

(If the installed Tauri reports a different identifier, verify against `src-tauri/gen/schemas/desktop-schema.json` — search it for `set-always-on-top` — and adjust. Gate = the dev build starts and the call is not rejected at manual launch.)

- [ ] **Step 2: Apply `alwaysOnTop` in `App`**

Edit `src/App.tsx`. Add the import:

```tsx
import { getCurrentWindow } from "@tauri-apps/api/window";
```

Add an effect (after the settings-fetch effect) that applies the flag on mount and on change:

```tsx
  // Apply always-on-top to the window whenever the setting changes (and on mount).
  useEffect(() => {
    getCurrentWindow()
      .setAlwaysOnTop(settings.alwaysOnTop)
      .catch((e) => console.error(e));
  }, [settings.alwaysOnTop]);
```

- [ ] **Step 3: Gate**

```bash
npx tsc --noEmit && npm run build
```
Expected: clean. (If `@tauri-apps/api/window`'s `getCurrentWindow`/`setAlwaysOnTop` shape differs in the installed version, adjust the call to the installed API; gate = clean build.)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/capabilities/default.json src/App.tsx
git commit -m "feat(ui): always-on-top window toggle wired to settings

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 6: Footer menu items (Automatic Drafts / Always On Top / Settings…)

Extend the Sidebar footer `DropdownMenu` with two `CheckboxItem`s bound to settings, a separator, and a "Settings…" item that opens the Settings dialog. The dialog open-state is lifted to `App` (Task 7 mounts the dialog there). Gate = tsc+build.

**Files:**
- Modify: `src/components/Sidebar.tsx`, `src/App.tsx`

- [ ] **Step 1: Add the settings props + open-settings callback to Sidebar**

Edit `src/components/Sidebar.tsx`. Add to `SidebarProps`:

```tsx
export interface SidebarProps {
  snapshot: Snapshot;
  selected: string;
  showArchived: boolean;
  hideCompleted: boolean;
  usesAutoDraft: boolean;
  alwaysOnTop: boolean;
  onSelect: (name: string) => void;
  onToggleHideCompleted: () => void;
  onToggleShowArchived: () => void;
  onToggleAutoDraft: () => void;
  onToggleAlwaysOnTop: () => void;
  onOpenSettings: () => void;
  onSnapshot: (snap: Snapshot) => void;
  onRequestConfirm: (req: ConfirmRequest) => void;
}
```

Destructure the new props in the function signature:

```tsx
export function Sidebar({
  snapshot, selected, showArchived, hideCompleted, usesAutoDraft, alwaysOnTop,
  onSelect, onToggleHideCompleted, onToggleShowArchived, onToggleAutoDraft, onToggleAlwaysOnTop,
  onOpenSettings, onSnapshot, onRequestConfirm,
}: SidebarProps) {
```

- [ ] **Step 2: Extend the footer `DropdownMenu`**

In the footer `DropdownMenu.Content`, after the existing "Show Archived" `CheckboxItem` (and before the existing `Separator`/"Add a Group"), add the two new checkbox items; then after "Add a Group" add a separator and the Settings item. The content becomes:

```tsx
          <DropdownMenu.Content>
            <DropdownMenu.CheckboxItem checked={hideCompleted} onCheckedChange={onToggleHideCompleted}>
              Hide Completed
            </DropdownMenu.CheckboxItem>
            <DropdownMenu.CheckboxItem checked={showArchived} onCheckedChange={onToggleShowArchived}>
              Show Archived
            </DropdownMenu.CheckboxItem>
            <DropdownMenu.CheckboxItem checked={usesAutoDraft} onCheckedChange={onToggleAutoDraft}>
              Automatic Drafts
            </DropdownMenu.CheckboxItem>
            <DropdownMenu.CheckboxItem checked={alwaysOnTop} onCheckedChange={onToggleAlwaysOnTop}>
              Always On Top
            </DropdownMenu.CheckboxItem>
            <DropdownMenu.Separator />
            <DropdownMenu.Item
              onSelect={() =>
                setPrompt({
                  title: "New Group",
                  label: "Group name",
                  initial: "",
                  submit: (v) => createGroup(v).then(onSnapshot).catch(console.error),
                })
              }
            >
              Add a Group
            </DropdownMenu.Item>
            <DropdownMenu.Separator />
            <DropdownMenu.Item onSelect={onOpenSettings}>Settings…</DropdownMenu.Item>
          </DropdownMenu.Content>
```

(Swift footer order is Automatic Drafts → Always On Top → Settings…; the Tauri footer keeps Hide Completed / Show Archived first since those already exist there. `onCheckedChange` passes a `boolean`; the toggle callbacks below ignore the argument and flip the stored value via `updateSettings`. If 3.3.0's `CheckboxItem` passes the next-checked boolean and you prefer to set it directly, adjust the App handlers to take that boolean — gate = clean build.)

- [ ] **Step 3: Lift dialog open-state + wire the new props in `App`**

Edit `src/App.tsx`. Add the dialog open-state (next to the other `useState` calls):

```tsx
  const [settingsOpen, setSettingsOpen] = useState(false);
```

Pass the new props to `<Sidebar …>`:

```tsx
      <Sidebar
        snapshot={snapshot}
        selected={view.selected}
        showArchived={view.showArchived}
        hideCompleted={view.hideCompleted}
        usesAutoDraft={settings.usesAutoDraft}
        alwaysOnTop={settings.alwaysOnTop}
        onSelect={(name) => setView((v) => ({ ...v, selected: name }))}
        onToggleHideCompleted={() => setView((v) => ({ ...v, hideCompleted: !v.hideCompleted }))}
        onToggleShowArchived={() => setView((v) => ({ ...v, showArchived: !v.showArchived }))}
        onToggleAutoDraft={() => updateSettings({ usesAutoDraft: !settingsRef.current.usesAutoDraft })}
        onToggleAlwaysOnTop={() => updateSettings({ alwaysOnTop: !settingsRef.current.alwaysOnTop })}
        onOpenSettings={() => setSettingsOpen(true)}
        onSnapshot={apply}
        onRequestConfirm={requestConfirm}
      />
```

(The `SettingsDialog` itself is added in Task 7; `settingsOpen`/`setSettingsOpen` are used there. Until then the open-state is set but unread, which `tsc` allows.)

- [ ] **Step 4: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green.

- [ ] **Step 5: Commit**

```bash
git add src/components/Sidebar.tsx src/App.tsx
git commit -m "feat(ui): footer menu items for auto-draft, always-on-top, settings

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 7: `SettingsDialog.tsx` + System Information tab

A new Radix `Dialog` + `Tabs` component with a single **System Information** tab: Version + Build (from `getVersion()`) and the store path (from `storePath()`). Controlled `open`/`onOpenChange` from `App`. The Command/Prompt tabs are added in 5C/5B.

> **Radix 3.3.0 latitude:** uses `Dialog` (controlled `open`/`onOpenChange`, `Content`, `Title`), `Tabs` (`Root`/`List`/`Trigger`/`Content`), and `Text`/`Flex`. If 3.3.0's `Tabs`/`Dialog` prop surface differs (e.g. `Tabs.Root value`/`defaultValue`), adjust to the installed API; gate is clean tsc+build.

**Files:**
- Create: `src/components/SettingsDialog.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Implement `SettingsDialog.tsx`**

```tsx
import { useEffect, useState } from "react";
import { Dialog, Flex, Tabs, Text } from "@radix-ui/themes";
import { getVersion } from "@tauri-apps/api/app";
import { storePath } from "../api/client";

export interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const [version, setVersion] = useState<string>("");
  const [path, setPath] = useState<string>("");

  // Load version + store path when the dialog opens (cheap; re-fetch is fine).
  useEffect(() => {
    if (!open) return;
    getVersion()
      .then(setVersion)
      .catch((e) => console.error(e));
    storePath()
      .then(setPath)
      .catch((e) => console.error(e));
  }, [open]);

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Content maxWidth="520px">
        <Dialog.Title>Settings</Dialog.Title>
        <Tabs.Root defaultValue="system">
          <Tabs.List>
            <Tabs.Trigger value="system">System Information</Tabs.Trigger>
            {/* Prompt tab added in 5B; Command tab added in 5C. */}
          </Tabs.List>
          <Tabs.Content value="system">
            <Flex direction="column" gap="3" mt="3">
              <Flex justify="between" gap="4">
                <Text size="2" color="gray">Version</Text>
                <Text size="2">{version || "Unavailable"}</Text>
              </Flex>
              <Flex direction="column" gap="1">
                <Text size="2" color="gray">Store Path</Text>
                <Text size="1" style={{ wordBreak: "break-all" }}>{path || "Unavailable"}</Text>
              </Flex>
            </Flex>
          </Tabs.Content>
        </Tabs.Root>
      </Dialog.Content>
    </Dialog.Root>
  );
}
```

(Parity: Swift `systemInformationSection` shows `Version` from `CFBundleShortVersionString` and `Build` from `CFBundleVersion`. Tauri's `getVersion()` returns the `tauri.conf.json` `version` — a single string — so this tab labels it `Version`; there is no separate Tauri "build" number to surface, so the Build row is omitted (or, if a build identifier is later added to `tauri.conf.json`, add a second row). The store path replaces Swift's data-location display.)

- [ ] **Step 2: Mount the dialog in `App`**

Edit `src/App.tsx`. Add the import:

```tsx
import { SettingsDialog } from "./components/SettingsDialog";
```

Mount it inside the top-level `<Flex>` (next to the confirm `AlertDialog.Root`), driven by the `settingsOpen` state from Task 6:

```tsx
      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
```

- [ ] **Step 3: Gate**

```bash
npx tsc --noEmit && npm run build
```
Expected: clean. (Report any `Tabs`/`Dialog` prop adjustment for 3.3.0.)

- [ ] **Step 4: Commit**

```bash
git add src/components/SettingsDialog.tsx src/App.tsx
git commit -m "feat(ui): settings dialog shell with System Information tab

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 8: Error dialogs (surface command rejections in an `AlertDialog`)

Add `errorMessage: string | null` + an `onError(msg)` callback in `App`, thread it alongside `onSnapshot` to the components that issue mutations, replace `.catch((e) => console.error(e))` with `.catch((e) => onError(String(e)))` on mutation promises, and render an `AlertDialog` host with a single OK dismiss. Gate = tsc+build.

**Files:**
- Modify: `src/App.tsx`, `src/components/Sidebar.tsx`, `src/components/DetailPane.tsx`, `src/components/TaskRow.tsx`, `src/components/InlineEditor.tsx`

- [ ] **Step 1: Add `errorMessage` + `onError` + the error host in `App`**

Edit `src/App.tsx`. Add state and callback (next to the other state/`useCallback`s):

```tsx
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const onError = useCallback((msg: string) => setErrorMessage(msg), []);
```

Update the top-level keyboard-handler and effect `.catch`es that already exist in `App` to use `onError` where a user-visible failure is meaningful (Cmd+N create, Cmd+Backspace delete):

```tsx
        createItem(target)
          .then((snap) => {
            setSnapshot(snap);
            const created = [...snap.items]
              .reverse()
              .find((i) => i.title === "" && i.status === "draft" && (!target || i.collection === target));
            if (created) {
              setFocusedId(created.id);
              setEditingTarget({ id: created.id, field: "title" });
            }
          })
          .catch((err) => onError(String(err)));
```

and

```tsx
          deleteItem(id).then(setSnapshot).catch((err) => onError(String(err)));
```

(Leave the initial `getSnapshot`/`getSettings`/`setAlwaysOnTop` `.catch(console.error)` as-is — those are startup/IPC plumbing, not user mutations, and the spec scopes error dialogs to "command rejections" from mutations.)

Pass `onError` to `Sidebar` and `DetailPane`:

```tsx
      <Sidebar
        /* …existing props… */
        onSnapshot={apply}
        onError={onError}
        onRequestConfirm={requestConfirm}
      />
      <DetailPane
        /* …existing props… */
        onSnapshot={apply}
        onError={onError}
        onRequestConfirm={requestConfirm}
      />
```

Add the error `AlertDialog` host after the confirm host (a separate host keyed by `errorMessage !== null`):

```tsx
      <AlertDialog.Root open={errorMessage !== null} onOpenChange={(o) => { if (!o) setErrorMessage(null); }}>
        <AlertDialog.Content maxWidth="420px">
          <AlertDialog.Title>Something went wrong</AlertDialog.Title>
          <AlertDialog.Description size="2">{errorMessage ?? ""}</AlertDialog.Description>
          <Flex gap="3" mt="4" justify="end">
            <AlertDialog.Action>
              <Button onClick={() => setErrorMessage(null)}>OK</Button>
            </AlertDialog.Action>
          </Flex>
        </AlertDialog.Content>
      </AlertDialog.Root>
```

(Parity: Swift sets `errorMessage = error.localizedDescription` at each mutation catch; here the command returns the `StoreError` parity string and `String(e)` is shown. The `*_if_current` no-op design already swallows benign stale races, so they never reach this host.)

- [ ] **Step 2: Thread `onError` through Sidebar and replace its `.catch`es**

Edit `src/components/Sidebar.tsx`. Add `onError: (msg: string) => void;` to `SidebarProps`, destructure it, and replace every `.catch(console.error)` on a mutation promise with `.catch((e) => onError(String(e)))`. The mutation sites are: `renameCollection`, `renameGroup`, `createCollection`, `setCollectionColor`, `setCollectionArchived`, `moveCollection`, `clearItems` (×2), `deleteGroup`, `deleteCollection`, and `createGroup` (the "Add a Group" item). Example:

```tsx
    submit: (v) => { renameCollection(c.name, v).then(onSnapshot).catch((e) => onError(String(e))); },
```

and inside the delete-group confirm:

```tsx
                      onConfirm: () => deleteGroup(group.name).then(onSnapshot).catch((e) => onError(String(e))),
```

- [ ] **Step 3: Thread `onError` through DetailPane → TaskRow → InlineEditor**

Edit `src/components/DetailPane.tsx`: add `onError: (msg: string) => void;` to `DetailPaneProps`, destructure it, pass `onError={onError}` to each `<TaskRow>`, and replace the `newTask` `createItem(...).catch(console.error)` with `.catch((e) => onError(String(e)))`.

Edit `src/components/TaskRow.tsx`: add `onError: (msg: string) => void;` to `TaskRowProps`, destructure it, pass `onError={onError}` to both `<InlineEditor>` instances, and replace each mutation `.catch(console.error)` (the `setStatus` advance/right-click, the status-menu `setStatus`, `moveItem`, `deleteItem`) with `.catch((e) => onError(String(e)))`.

Edit `src/components/InlineEditor.tsx`: add `onError: (msg: string) => void;` to `InlineEditorProps`, destructure it, and replace the `console.error(e)` calls:
- In `save`'s `catch (e) { console.error(e); }` → `catch (e) { onError(String(e)); }`.
- In the `Commit` title path's `catch (e) { console.error(e); }` (added in Task 4) → `catch (e) { onError(String(e)); }`.
- In `execute`'s inline `.catch(console.error)` calls (the `Split`/`MergeIntoPrevious`/`DeleteEmpty` mutation promises) → `.catch((e) => onError(String(e)))`.

- [ ] **Step 4: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green. (Report any `AlertDialog` 3.3.0 prop adjustment; the confirm host already uses this API, so none expected.)

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/components/Sidebar.tsx src/components/DetailPane.tsx src/components/TaskRow.tsx src/components/InlineEditor.tsx
git commit -m "feat(ui): surface command rejections in an error dialog

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Done criteria

- `cargo test -p pond-tauri` green (incl. the 5 `settings::tests`); `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- `npx vitest run` green (incl. `autodraft.test.ts` + the two new `client.test.ts` cases); `npx tsc --noEmit` clean; `npm run build` succeeds.
- Manual `cargo tauri dev` launch (human check): the footer menu shows Automatic Drafts / Always On Top / Settings…; toggling them persists across relaunch; editing a non-draft title drops it to draft (auto-draft on) and confirming promotes to ready; a fresh Cmd+N draft promotes to ready on confirm; Always On Top floats the window; the Settings dialog shows the version + store path; a forced command error shows the error dialog; the last selected collection is restored on relaunch.
