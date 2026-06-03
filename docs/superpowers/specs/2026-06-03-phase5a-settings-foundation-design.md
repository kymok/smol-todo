# Phase 5A: Settings Foundation + Auto-Draft + Always-On-Top + Error Dialogs — Design

- **Date:** 2026-06-03
- **Status:** Approved design
- **Phase:** 5A of 5 (Phase 5 split into 5A/5B/5C; see `2026-06-02-tauri-radix-migration-design.md` §9.5)
- **Builds on:** Phases 1–4 complete (read-only shell + full mutation/editing GUI).

## 1. Overview

Introduce the app-settings store and the behaviors that depend on it, and surface mutation
errors in the UI. This is the foundation for 5B (prompts/export/clipboard) and 5C
(CLI-install/file-drop/bulk-status). No `pond-core` changes — settings are GUI-only and live
in `pond-tauri`.

### Goals
- An app-settings store (`settings.json`) with `get_settings`/`set_settings` IPC and React state.
- **Auto-draft** on title edits (default on): editing drops a task to `draft`; confirming
  promotes to `ready` — gated by `usesAutoDraft`, matching the Swift app.
- **Always-on-top** window toggle (default off), persisted.
- Footer-menu items: Automatic Drafts, Always On Top, Settings…
- A **Settings dialog** shell with the **System Information** tab (version/build + store path).
- **Error dialogs**: command rejections surfaced in an `AlertDialog` instead of `console.error`.

### Non-goals (deferred)
- **5B:** per-collection prompt editor, Copy Prompt / Copy CLI Command, export save-dialog,
  clipboard + dialog plugins, the Settings **Prompt** tab.
- **5C:** CLI-install UI + `taskpond` sidecar packaging, file-drop-to-create, bulk-status
  dialog, the Settings **Command** tab.
- Window-frame/size persistence (Tauri manages window state; the master spec lists only
  `lastSelectedCollection`, not frame geometry).

## 2. Confirmed decisions

1. **Settings live in `pond-tauri`** (GUI-only; the CLI doesn't need them), in a `settings.rs`
   module managing `settings.json` in the OS config dir via the `directories` crate (same one
   `pond-core` uses), with atomic temp+rename writes. Held in Tauri state behind a `Mutex`.
2. **Whole-object commands:** `get_settings() -> Settings`, `set_settings(Settings) -> Settings`.
   The frontend reads, changes a field, and writes the whole object back (simpler than typed
   per-field setters). `set_settings` returns the persisted value.
3. **Auto-draft applies to title edits only** (notes never auto-draft — matches Swift). The
   status decision is a pure, unit-tested helper; the InlineEditor executor applies it.
4. **Error model:** surface command rejections in a Radix `AlertDialog`. The `*_if_current`
   no-op design already swallows benign stale races, so no Swift "ignore-list" replication is
   needed.
5. **Settings dialog** is a Radix `Dialog` + `Tabs`; 5A builds the shell + System Information
   tab; 5B/5C add the Prompt and Command tabs.
6. Radix defaults only; no visual/DOM tests (verification = logic + build gates + manual launch).

## 3. Architecture

```
src-tauri/src/
├─ settings.rs    (NEW) Settings struct (serde camelCase) + load/save (settings.json, atomic) + Mutex state helper
├─ commands.rs    + get_settings / set_settings #[tauri::command] wrappers
└─ main.rs        app.manage(Mutex<Settings>) loaded at startup; register the 2 commands; window capability for set_always_on_top

src/
├─ api/
│  ├─ client.ts   + getSettings() / setSettings(s) wrappers
│  └─ types.ts    + Settings interface (mirrors the DTO)
├─ state/
│  └─ autodraft.ts (NEW) pure autoDraftStatus(...) helper (+ test)
├─ components/
│  ├─ Sidebar.tsx     footer: Automatic Drafts / Always On Top / Settings… items
│  ├─ SettingsDialog.tsx (NEW) Dialog + Tabs shell with the System Information tab
│  └─ InlineEditor.tsx   apply auto-draft status on title edit/confirm (reads usesAutoDraft)
└─ App.tsx        settings state (fetch on mount); apply always-on-top via the window API; errorMessage + error AlertDialog host; restore lastSelectedCollection
```

## 4. Settings store

- **`Settings`** (serde `rename_all="camelCase"`):
  `uses_auto_draft: bool` (default `true`), `always_on_top: bool` (default `false`),
  `default_prompt_template: String` (default `""`), `last_selected_collection: Option<String>`
  (default `None`).
- **File:** `<config_dir>/pond/settings.json` (config dir via `directories::ProjectDirs` like
  `pond-core::paths`; reuse the same app identifiers). Missing/corrupt file → defaults (and a
  fresh write on first `set_settings`). Atomic temp+rename, pretty + sorted (reuse
  `pond-core`'s JSON helper or `serde_json` directly).
- **Serialization:** `serde_json` pretty-printed (this is `pond-tauri`-local config, not the
  shared store format, so it doesn't need `pond-core`'s sorted-key encoder).
- **State:** `app.manage(Mutex<Settings>)` loaded once at startup.
- **Commands:** `get_settings()` returns the current `Settings`; `set_settings(Settings)`
  replaces it, persists to disk, returns the persisted value. Errors map to `String`.
- **Frontend:** `App` fetches on mount into React state; a `updateSettings(patch)` helper
  merges + calls `setSettings`. `lastSelectedCollection` is written when the selection changes
  and read on mount to restore the selection.

## 5. Auto-draft (port of `DetailView.saveTitle`/`confirmTitle`)

A pure helper in `state/autodraft.ts`:

```
autoDraftStatus({ usesAutoDraft, currentStatus, phase, titleChanged }): TaskStatus | undefined
  // phase: "edit" (autosave / blur)  |  "confirm" (Cmd+Enter or Tab)
  if (phase === "confirm") {
    if (currentStatus === "draft") return "ready";       // draft → ready on confirm, always
    return usesAutoDraft ? "ready" : undefined;          // non-draft → ready when auto-draft on
  }
  // phase === "edit"
  if (!titleChanged) return undefined;                   // no change → no status change
  if (currentStatus === "draft") return undefined;       // a draft stays draft while editing
  return usesAutoDraft ? "draft" : undefined;            // non-draft → draft when auto-draft on
```

Maps to Swift: `autoDraftEditStatus = usesAutoDraft ? .draft : nil`,
`autoDraftConfirmationStatus = usesAutoDraft ? .ready : nil`, and
`confirmationStatus = item.status == .draft ? .ready : autoDraftConfirmationStatus`.

**InlineEditor wiring** (title field only):
- The editor receives `usesAutoDraft` (threaded from settings via App→DetailPane→TaskRow).
- **Edit phase** = the 500 ms debounced autosave and blur-save. When it saves a changed title,
  compute `autoDraftStatus({phase:"edit", ...})`; if defined, include it as the `status` arg of
  `updateItem` (alongside the title). When `undefined`, send title only (today's behavior).
- **Confirm phase** = the reducer's `Commit` intent (Cmd+Enter / Tab). Compute
  `autoDraftStatus({phase:"confirm", ...})`; include the resulting `status` in the `updateItem`.
- Notes: unchanged (no auto-draft).
- A brand-new Cmd+N draft is already `draft`; editing keeps it draft (edit phase returns
  `undefined` for a draft), and confirming promotes it to `ready` (confirm phase, draft→ready).

## 6. Always-on-top, footer menu, Settings dialog

- **Always-on-top:** on mount and whenever `alwaysOnTop` changes, call
  `getCurrentWindow().setAlwaysOnTop(alwaysOnTop)` (`@tauri-apps/api/window`). Add the matching
  window permission to `src-tauri/capabilities/default.json` (e.g. `core:window:allow-set-always-on-top`).
- **Footer menu** (extend the Phase 4 footer `DropdownMenu`): `CheckboxItem` "Automatic Drafts"
  (↔ `usesAutoDraft`), `CheckboxItem` "Always On Top" (↔ `alwaysOnTop`), a separator, and an
  item "Settings…" that opens the Settings dialog. (Hide Completed / Show Archived / Add a
  Group remain.)
- **Settings dialog** (`SettingsDialog.tsx`): Radix `Dialog` + `Tabs`. 5A ships the **System
  Information** tab: app version + build (from the Tauri app — `getVersion()` / `tauri.conf.json`
  version) and the store path (`POND_STORE` or the default). Command/Prompt tabs are added in
  5C/5B.

## 7. Error dialogs

- `App` holds `errorMessage: string | null` and exposes an `onError(msg: string)` callback,
  threaded down alongside `onSnapshot` (the same pattern Phase 4 used for the snapshot/confirm
  callbacks). Components replace their `.catch((e) => console.error(e))` on mutation promises
  with `.catch((e) => onError(String(e)))`.
- A Radix `AlertDialog` (separate from the Phase 4 confirm host, or a shared host keyed by
  mode) shows the message with a single **OK** dismiss. The message is the `StoreError` parity
  string the command already returns.
- Benign stale races don't surface here because the `*_if_current` commands return `Ok` (no-op)
  rather than erroring.

## 8. Testing / verification

- **Rust (`pond-tauri`):** `settings.rs` tests against a tempdir — defaults when the file is
  absent, round-trip `set`→`get`, and overwrite/persist. (Inject the settings path for tests,
  mirroring `cli_install`'s injectable paths.)
- **Frontend logic (Vitest):** `autoDraftStatus(...)` — every branch (edit/confirm ×
  draft/non-draft × usesAutoDraft on/off × titleChanged).
- **Gates:** `cargo test && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt
  --all -- --check` and `npx vitest run && npx tsc --noEmit && npm run build` stay green.
- **No visual/DOM tests** — always-on-top, the Settings dialog, the footer toggles, and the
  error dialog are verified at manual launch.

## 9. References (source of truth)
- `Sources/PondApp/AppModel.swift` — `usesAutoDraft`, `autoDraftEditStatus`/
  `autoDraftConfirmationStatus`, `errorMessage`.
- `Sources/PondApp/DetailView.swift` — `saveTitle`/`confirmTitle`/`confirmationStatus`.
- `Sources/PondApp/SidebarView.swift` — footer toggles (Automatic Drafts / Always On Top /
  Settings); `Sources/PondApp/ContentView.swift` + `WindowControllers.swift` — always-on-top
  (`window.level = .floating`).
- `Sources/PondApp/SettingsView.swift` — Settings tabs; System Information (Version/Build).
- Master spec `2026-06-02-tauri-radix-migration-design.md` §4 (App settings), §5 (IPC), §9.5.
