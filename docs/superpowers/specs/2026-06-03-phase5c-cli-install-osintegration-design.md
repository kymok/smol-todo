# Phase 5C: CLI Install + OS Integration (final) — Design

- **Date:** 2026-06-03
- **Status:** Approved design
- **Phase:** 5C of 5 (final sub-phase of Phase 5; see `2026-06-02-tauri-radix-migration-design.md` §9.5)
- **Builds on:** Phases 1–4 + 5A + 5B complete. Completing 5C finishes the SwiftUI → Tauri migration.

## 1. Overview

The last sub-phase: the macOS **CLI-install** feature (Settings Command tab, with `taskpond`
bundled as a Tauri sidecar), **file-drop-to-create**, and the **bulk-status** dialog. The
install and bulk-status *logic* already exist in `pond-core` (`cli_install::Installer`,
`set_statuses`); 5C is the sidecar packaging + IPC + UI wiring.

### Goals
- Install / uninstall `~/.local/bin/taskpond` from the GUI (Settings → Command tab), with
  `taskpond` shipped in the app bundle as a sidecar.
- Drop files onto the task list to create one task per file (titled with the filename).
- Bulk-remap statuses within a collection ("Change Statuses…").

### Non-goals
- Windows install (the installer is `cfg(unix)`, macOS-primary — matching Phases 1–2).
- Anything already shipped in 5A/5B.
- This is the final phase; no features are deferred past it.

## 2. Confirmed decisions

1. **Full sidecar packaging** (user decision). `taskpond` ships as a Tauri `externalBin`
   sidecar; a build-copy step places the built CLI where Tauri expects it. The
   **distributed-`.app` install is verified by the user** via `cargo tauri build` (a full
   bundle can't be produced/verified in the dev environment); the build-copy script + dev
   behavior + gates are verified here.
2. **Install target = `current_exe().parent()/taskpond`** — the binary alongside the app
   executable. In `cargo tauri dev` that's `target/debug/taskpond` (built by the workspace);
   in a bundle it's the sidecar next to the main exe. `Installer::new(default_link,
   resolved_target, default_record)`.
3. **Install commands are `cfg(unix)`** (macOS-primary). On a non-unix build the commands
   would report "unsupported"; in practice the target is macOS.
4. **Bulk-status = per-collection status remap** (matching Swift's `.collection` scope): a
   dialog maps each *currently-present* status in the collection to a replacement →
   `set_statuses(replacements, collection)`.
5. **File-drop creates a Draft titled with the filename** (port of Swift
   `createTaskFromDroppedFile` → `createTask(title: filename, status: .draft)`). Implemented by
   extending `create_item` with an optional `title` (Cmd+N still creates an empty Draft).
6. Radix defaults only; no visual/DOM tests (logic in Rust/helpers is unit-tested; the Command
   tab, file-drop, and bulk-status dialog are verified at manual launch; the real install in a
   bundle is the user's `cargo tauri build` check).

## 3. Architecture

```
src-tauri/
├─ tauri.conf.json   + bundle.externalBin ["binaries/taskpond"]; beforeDevCommand/beforeBuildCommand call the sidecar build-copy
├─ binaries/         (gitignored) taskpond-<target-triple> — produced by the build-copy step
├─ Cargo.toml        (no new deps; cli_install is already in pond-core)
└─ src/
   ├─ install.rs     (NEW) resolve_target() (current_exe sibling) + InstallStatusDto + From<&InstallStatus>
   ├─ commands.rs     + cli_install_status / cli_install / cli_uninstall (cfg(unix)); set_statuses; create_item gains optional title
   ├─ mutations.rs    + set_statuses(store, replacements, collection) -> SnapshotDto; create_item title arg
   └─ main.rs         register the new commands
scripts/
└─ build-sidecar.mjs (NEW) cargo build -p taskpond-cli (profile) + copy target/<profile>/taskpond → src-tauri/binaries/taskpond-<triple>

src/
├─ api/client.ts     + cliInstallStatus/cliInstall/cliUninstall, setStatuses, createItem(title?), fileDrop wiring
├─ api/types.ts      + InstallStatus DTO type
├─ state/bulkStatus.ts (NEW) pure presentStatuses(snapshot, collection) -> TaskStatus[]
├─ components/
│  ├─ SettingsDialog.tsx     + Command tab (install UI)
│  ├─ BulkStatusDialog.tsx   (NEW) present-status → replacement Select grid
│  ├─ Sidebar.tsx            collection menu: "Change Statuses…"
│  └─ App.tsx                drag-drop subscription → create_item per file; host BulkStatusDialog
```

## 4. Sidecar packaging

- **`tauri.conf.json`:** `bundle.externalBin: ["binaries/taskpond"]` (Tauri resolves to
  `binaries/taskpond-<target-triple>` and bundles it next to the main binary).
- **Build-copy step** (`scripts/build-sidecar.mjs`, invoked from Tauri's hooks): run
  `cargo build -p taskpond-cli` (debug for dev, release for build), read the host target triple
  (`rustc -vV` → the `host:` line), copy `target/<profile>/taskpond` →
  `src-tauri/binaries/taskpond-<triple>` (chmod +x). Wire into `tauri.conf.json`:
  `beforeDevCommand` = build-copy (debug) + the existing `npm run dev`; `beforeBuildCommand` =
  build-copy (release) + the existing `npm run build`. This satisfies Tauri's externalBin
  presence check for both `dev` and `build`.
- **`.gitignore`:** add `/src-tauri/binaries/` (built artifact, never committed).
- Runtime resolution is independent of `src-tauri/binaries/`: it uses the `current_exe`
  sibling (dev: `target/debug/taskpond`; bundle: the sidecar Tauri copied next to the exe).

## 5. CLI-install commands (`install.rs` + `commands.rs`, `cfg(unix)`)

- `resolve_taskpond_target() -> PathBuf` = `std::env::current_exe()?.parent()?.join("taskpond")`.
- `InstallStatusDto` (serde camelCase): `link_path: String`, `target_path: String`,
  `installed: bool`, `conflict_description: Option<String>` (`skip_serializing_if`),
  `install_directory_is_in_path: bool`, `can_uninstall: bool`, `can_install: bool`,
  `path_hint: String`. Built by a helper `dto_from(status: &InstallStatus, path_hint: String)
  -> InstallStatusDto` that copies the `InstallStatus` fields, sets `can_install =
  status.can_install()`, and carries `path_hint` from the argument (`path_hint` is the constant
  `Installer::path_hint()` — it is NOT a field of `InstallStatus`, so a bare
  `From<&InstallStatus>` can't produce it).
- Commands (each builds `Installer::new(home/.local/bin/taskpond, resolve_taskpond_target(),
  data_dir/cli-install.json)` — reuse the default link/record from `Installer::with_defaults`
  but override the target):
  - `cli_install_status() -> Result<InstallStatusDto, String>` → `installer.status()`.
  - `cli_install() -> Result<InstallStatusDto, String>` → `installer.install()?; status`.
  - `cli_uninstall() -> Result<InstallStatusDto, String>` → `installer.uninstall()?; status`.

## 6. Settings Command tab (frontend)

Port of Swift `commandSection`. Add a **Command** `Tabs.Trigger`/`Content` to `SettingsDialog`:
- **Link**: `status.linkPath` (selectable text).
- **Status**: installed → "Installed" (green); else the `conflictDescription` or "Not installed".
- **Add to PATH** (only when `!installDirectoryIsInPath`): `status.pathHint` (monospace) + a
  copy button (`copyText`).
- Buttons: **Install** / **Reinstall** (label "Reinstall" when `installed`; disabled when
  `!installed && !canInstall`) → `cliInstall`; **Uninstall** (disabled when `!canUninstall`) →
  `cliUninstall`. Both refresh the status from the returned DTO.
- Fetch `cliInstallStatus()` when the Command tab opens (and after install/uninstall).

## 7. Bulk-status (command + dialog)

- **`set_statuses(replacements: HashMap<TaskStatus,TaskStatus>, collection: String) -> Snapshot`**
  command → `mutations::set_statuses(store, &replacements, &[], Some(collection))` (→ pond-core
  `set_statuses`; ids empty, scoped to the collection). Returns the rebuilt snapshot. On the
  wire `replacements` is a JS object of status-string → status-string (e.g.
  `{ "ready": "completed" }`), deserializing to `HashMap<TaskStatus, TaskStatus>` via the
  kebab/lowercase rawValues — the plan verifies serde map-key deserialization of the enum.
- **`state/bulkStatus.ts`** (pure, tested): `presentStatuses(snapshot, collection) ->
  TaskStatus[]` — the distinct statuses among that collection's items, in status order.
- **`BulkStatusDialog.tsx`**: for each present status, a row "`<status>` → `<Select replacement>`"
  (default = the same status = unchanged); Confirm builds the `replacements` map from the rows
  that changed and calls `setStatuses(map, collection)` → onSnapshot. Opened from the
  collection menu **"Change Statuses…"** (App holds `bulkStatusCollection: string | null`).

## 8. File-drop-to-create (frontend)

- Extend **`create_item`** with an optional `title`: `mutations::create_item(store, collection:
  Option<&str>, title: Option<&str>)` → `add(title.unwrap_or(""), collection|DEFAULT_COLLECTION,
  None, true, Draft)`. The command + `createItem` wrapper gain an optional `title`; existing
  callers (Cmd+N, New Task) pass none (empty Draft — unchanged).
- **`App.tsx`**: subscribe to Tauri's window drag-drop event (`getCurrentWindow().onDragDropEvent`
  or the `tauri://drag-drop` event); on a drop carrying file paths, for each path call
  `createItem(targetCollection, basename(path))` (target = the selected collection, or default
  on "All"), then replace the snapshot from the last result. Errors via `onError`.
- Enable `dragDropEnabled` on the window in `tauri.conf.json` if not already on; add any
  required capability (verify the drag-drop event permission against the generated schema, as in
  5A/5B).

## 9. Testing / verification

- **Rust (`pond-tauri`):** `InstallStatusDto` mapping (`dto_from` copies the `InstallStatus`
  fields, derives `can_install`, carries `path_hint`); `resolve_taskpond_target` (sibling of a given exe path
  — factor the join so it's testable without a real `current_exe`); `set_statuses` command
  (snapshot reflects the remap); extended `create_item` (title path creates the titled Draft;
  no-title path still empty Draft). (`cli_install` install/uninstall is pond-core-tested;
  `set_statuses` core logic is pond-core-tested.)
- **Frontend logic (Vitest):** `presentStatuses` (distinct statuses, ordering, empty); client
  wrappers (mocked invoke).
- **Gates** stay green. **No visual/DOM tests** — the Command tab, file-drop, and bulk-status
  dialog are manual-launch checks; the **real install in a packaged app is the user's
  `cargo tauri build` verification**, and the build-copy script is verified by a clean
  `cargo tauri dev` start.

## 10. References (source of truth)
- `Sources/PondApp/SettingsView.swift` — `commandSection` (Link / Status / Add-to-PATH /
  Reinstall / Uninstall).
- `Sources/PondApp/AppModel.swift` — `cliStatus`/`installCLI`/`uninstallCLI`/`pathHint`;
  `createTask` (default `.draft`).
- `Sources/PondApp/CollectionMenus.swift` — bulk-status scope; `Sources/PondApp/ContentView.swift`
  — `BulkStatusChangeSheet` (present-status → replacement).
- `Sources/PondApp/DetailView.swift` — `createTaskFromDroppedFile`, the `onDrop` handlers.
- `crates/pond-core/src/cli_install.rs` — `Installer`, `InstallStatus`, `with_defaults`,
  `status`/`install`/`uninstall`/`path_hint`.
- `crates/pond-core/src/store.rs` — `set_statuses`, `add`.
- Master spec `2026-06-02-tauri-radix-migration-design.md` §5 (IPC), §6 (Settings/bulk),
  §7 (file-drop), §8 (installer sidecar).
