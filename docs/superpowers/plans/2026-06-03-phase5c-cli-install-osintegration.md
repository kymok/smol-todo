# Phase 5C: CLI Install + OS Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the SwiftUI → Tauri migration by wiring the last cluster from the approved 5C design: ship `taskpond` as a Tauri **sidecar** (`externalBin` + a build-copy step), expose the macOS **CLI-install** feature in Settings (a **Command** tab — install/uninstall `~/.local/bin/taskpond`), add **file-drop-to-create** (drop files onto the window → one Draft per file, titled with the filename), and a per-collection **bulk-status** remap dialog ("Change Statuses…"). The install + bulk-status *logic* already lives in `pond-core` (`cli_install::Installer`, `TaskStore::set_statuses`); 5C is the sidecar packaging + IPC + UI wiring.

**Architecture:** `pond-core` stays the single source of truth. The install logic is `pond_core::cli_install::Installer` (unix-only); the bulk-status remap is `TaskStore::set_statuses`. The established `pond-tauri` seam holds: pure/testable functions take plain inputs (`install::target_beside`, `install::dto_from`) or `&TaskStore` (`mutations::set_statuses`, `mutations::create_item`); thin `#[tauri::command]` wrappers in `commands.rs` do `State` access + `Result<_, String>` mapping and are registered in `main.rs`. The CLI-install commands are `#[cfg(unix)]` (macOS-primary, matching `pond_core::cli_install`'s `#![cfg(unix)]`). At runtime the install target is the binary **beside the app executable** (`current_exe().parent()/taskpond`): in `cargo tauri dev` that is `target/debug/taskpond` (built by the workspace); in a bundle it is the sidecar Tauri copies next to the main exe. `Installer` builds an `InstallStatus` whose fields map 1:1 to an `InstallStatusDto` (camelCase) plus a derived `canInstall` (a method, not a field) and a `pathHint` (a method on `Installer`, not on `InstallStatus`) — so a helper `dto_from(&InstallStatus, path_hint: String)` builds the DTO. The frontend keeps its single `invoke` site (`api/client.ts`); the Command tab calls the client directly and refreshes from the returned DTO; the bulk-status dialog computes its rows from a pure `state/bulkStatus.ts` helper and posts a changed-only `Record<string,string>` map; the file-drop subscriber lives in `App.tsx` (the existing single `getCurrentWindow()` consumer). UI errors route through the existing `onError` seam from 5A.

**Tech Stack:** Rust 1.96.0 (pinned), Tauri v2 (`tauri` 2.x), `serde`/`serde_json`, `tempfile` (dev); Node (the build-copy script is a plain `.mjs` run by Tauri's `beforeDevCommand`/`beforeBuildCommand`); Vite + React 18 + TypeScript + `@radix-ui/themes` 3.3.0 + `@radix-ui/react-icons`; `@tauri-apps/api` v2 (`core`, `window`); Vitest. npm. **No new crates or npm packages** — `cli_install` is already in `pond-core`, drag-drop is in `@tauri-apps/api/window`, and the clipboard plugin (for the PATH-hint copy) was added in 5B.

---

## Conventions (read this section before `## File Structure`)

Every task obeys these. They are not repeated per step.

- **Branch:** work on the existing `tauri-radix-migration` branch. Do **not** create a new branch and do **not** set an upstream.
- **Rust toolchain:** pinned `1.96.0` (already in `rust-toolchain.toml`). Run all `cargo`/`npm`/`npx`/`node` commands from the repo root (the Vite root is the repo root).
- **Per Rust task gate:** `cargo fmt --all` then `cargo clippy --workspace --all-targets -- -D warnings` must be clean, and `cargo test -p pond-tauri` green.
- **Per frontend task gate:** `npx tsc --noEmit` clean, `npm run build` succeeds (it runs `tsc --noEmit && vite build`), `npx vitest run` green.
- **Imports/`use` at the top:** ALL `import` (TS) and `use` (Rust) statements live at the top of the file. In Rust test modules, all `use` go at the top of `mod tests` (i.e. directly under `#[cfg(test)] mod tests {`). No mid-file imports.
- **Radix Themes defaults only:** stock Radix parts, built-in named palette, no theme customization. The TSX below targets the installed `@radix-ui/themes` **3.3.0** API (already used across Phases 3–5B: `Dialog`, `Tabs`, `TextArea`, `Select`, `ContextMenu.Sub`/`SubTrigger`/`SubContent`, `Flex`, `Text`, `Button`, `Link`, `Code`). Where a sample's component/prop shape differs from what 3.3.0 actually exports, **adjust the usage to the installed API** — the gate is a clean `tsc --noEmit` + `npm run build` (report any adjustment in the task's commit/notes). This is the established Phase 3–5B latitude, **not** a placeholder to leave logic unwritten.
- **No visual / DOM / screenshot tests.** Verification is logic unit tests (the `install.rs` + `mutations` Rust tests, the `client.test.ts` mocked-invoke assertions, the `bulkStatus.test.ts` pure assertions) + the build/typecheck gates + manual `cargo tauri dev` launch (human check). Do **not** add `@testing-library`/`jsdom` render tests for the Command tab, the bulk-status dialog, or the file-drop wiring.
- **Command (invoke) names** must equal exactly what the frontend `client.ts` wrapper passes to `invoke` (`cli_install_status`, `cli_install`, `cli_uninstall`, `set_statuses`, `create_item`).
- **Commit trailer:** every commit message ends with a trailing line:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`

### IMPORTANT — Task 1 (sidecar) verification scope

Task 1 verification is **NOT** a full `cargo tauri dev` / `cargo tauri build` (those need the Tauri CLI and, for `build`, produce a signed bundle — that is the **USER's** verification per design decision 1). For Task 1 the implementer verifies:
1. The build-copy **script runs** (`node scripts/build-sidecar.mjs debug`) and produces `src-tauri/binaries/taskpond-<host-triple>` that is executable (`-x`).
2. `cargo build -p pond-tauri` stays clean (the sidecar config does not break the Rust build).
3. `tauri.conf.json` parses as valid JSON (e.g. `node -e "JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json','utf8'))"` prints nothing and exits 0).

The full `dev`/`build` run (which exercises Tauri's externalBin presence check and bundling) is the user's check.

### Verified facts (source of truth — do not guess)

- **`cli_install` is unix-only and NOT re-exported at the crate root.** `crates/pond-core/src/lib.rs` has `#[cfg(unix)] pub mod cli_install;` — there is **no** `pub use cli_install::...`. So the path is **`pond_core::cli_install::{Installer, InstallStatus}`** (the module is public, the types are `pub`). Both `Installer` and `InstallStatus` are public; `cli_install.rs` itself starts with `#![cfg(unix)]`. Therefore all Rust code that names them must be `#[cfg(unix)]`-gated (Task 2).
- **`InstallStatus` fields (`crates/pond-core/src/cli_install.rs`):**
  ```rust
  pub struct InstallStatus {
      pub link_path: PathBuf,
      pub target_path: PathBuf,
      pub installed: bool,
      pub conflict_description: Option<String>,
      pub install_directory_is_in_path: bool,
      pub can_uninstall: bool,
  }
  impl InstallStatus { pub fn can_install(&self) -> bool { /* … */ } }
  ```
  **`can_install` is a METHOD, not a field.** `path_hint` is **not** on `InstallStatus` at all — it is `Installer::path_hint(&self) -> String` (returns the constant `export PATH="$HOME/.local/bin:$PATH"`). So the DTO helper must take the `path_hint` as a separate argument; a bare `From<&InstallStatus>` cannot produce `canInstall` + `pathHint` together. `link_path`/`target_path` are `PathBuf` → the DTO carries them as `String` via `to_string_lossy().to_string()`.
- **`Installer` constructors (`cli_install.rs`):** `Installer::new(link_path: PathBuf, target_path: PathBuf, record_path: PathBuf) -> Self` (explicit paths — used by the GUI + tests); `Installer::with_defaults() -> Self` (link `~/.local/bin/taskpond`, record `<data-dir>/cli-install.json`, target = `current_exe()` fallback `taskpond`). `status(&self) -> InstallStatus`, `install(&self) -> Result<()>`, `uninstall(&self) -> Result<()>`, `path_hint(&self) -> String`. The install/uninstall happy paths require the target to be an existing executable file (`install` errors "was not found" otherwise) — the round-trip test therefore creates an executable temp file as the target.
- **`TaskStore::set_statuses` (`crates/pond-core/src/store.rs:460`):** `pub fn set_statuses(&self, replacements: &HashMap<TaskStatus, TaskStatus>, ids: &[String], collection: Option<&str>) -> Result<Vec<TaskItem>>`. It filters out no-op pairs (`a == b`) and only touches items whose current status is a key. For a per-collection remap: `store.set_statuses(replacements, &[], Some(collection))`. Returns the changed items (we discard them and rebuild the snapshot).
- **`TaskStore::add` (`store.rs:176`):** `pub fn add(&self, title: &str, collection: &str, requested_id: Option<&str>, allow_empty_title: bool, status: TaskStatus) -> Result<TaskItem>`. `mutations::create_item` already calls `store.add("", target, None, true, TaskStatus::Draft)`.
- **`paths::data_directory` (`crates/pond-core/src/paths.rs:9`):** `pub fn data_directory() -> PathBuf` (the `pond` ProjectDirs data dir, fallback `PathBuf::from("pond")`). The CLI-install record path is `pond_core::paths::data_directory().join("cli-install.json")` — matching `Installer::with_defaults`.
- **`TaskStatus` (`crates/pond-core/src/model.rs:7`):** `#[serde(rename_all = "kebab-case")]` with `#[serde(rename = "in-progress")] InProgress` and `#[serde(rename = "on-hold")] OnHold`. The variant order (also `TaskStatus::all() -> [TaskStatus; 7]`) is **Draft, Ready, InProgress, Completed, OnHold, Rejected, Aborted**. Wire forms: `draft, ready, in-progress, completed, on-hold, rejected, aborted` — which is exactly the order of `src/api/types.ts`'s `TaskStatus` union, so the frontend `presentStatuses` can use that literal order as the canonical ordering. A `HashMap<TaskStatus, TaskStatus>` deserializes from a JSON object keyed by those wire strings (proven by a Task 3 test).
- **`mutations::create_item` (`src-tauri/src/mutations.rs:11`) — CURRENT:** `pub fn create_item(store: &TaskStore, collection: Option<&str>) -> Result<SnapshotDto>` → `store.add("", target, None, true, TaskStatus::Draft)`. Task 4 extends it with an optional `title`. The command wrapper (`src-tauri/src/commands.rs:34`) is `pub fn create_item(store: State<TaskStore>, collection: Option<String>) -> std::result::Result<SnapshotDto, String>`. **There are no other internal callers of `mutations::create_item`** (only the command wrapper calls it; grep confirms).
- **`build_snapshot` (`src-tauri/src/commands.rs:12`):** `pub fn build_snapshot(store: &TaskStore) -> Result<SnapshotDto>` — every mutation returns its result. Imported into `mutations.rs` as `use crate::commands::build_snapshot;`.
- **`mutations.rs` already has a `#[cfg(test)] mod tests`** (top block: `use super::*; use pond_core::export::ExportFormat; use pond_core::{CollectionColor, TaskStatus, DEFAULT_GROUP}; use tempfile::tempdir;`). Add `use std::collections::HashMap;` to that block for Task 3. Do **not** create a second `mod tests`.
- **`commands.rs` top `use` block (current):** `use crate::dto::{CollectionGroupSummaryDto, CollectionSummaryDto, SnapshotDto}; use crate::mutations; use crate::prompt; use crate::settings::{self, Settings}; use pond_core::export::ExportFormat; use pond_core::{CollectionColor, Result, TaskItem, TaskStatus, TaskStore}; use std::collections::HashMap; use std::sync::Mutex; use tauri::State;` — `HashMap`, `TaskStatus`, `TaskStore` are already imported (Task 3 reuses them).
- **`main.rs` builder/handler (current):** plugins `tauri_plugin_clipboard_manager::init()` + `tauri_plugin_dialog::init()` already chained; module list is `commands, dto, mutations, prompt, settings, watcher`; `generate_handler!` ends with `commands::export_collection,`. Task 2 adds `mod install;` and registers the three cli-install commands; Task 3/4 register `set_statuses` (Task 4 changes no registration — `create_item` is already registered).
- **`src/api/client.ts` (current):** single `invoke` site. `createItem(collection?: string): Promise<Snapshot>` → `invoke("create_item", { collection: collection ?? null })`. The 5B prompt/export wrappers exist. The clipboard helper `src/lib/clipboard.ts` exports `copyText(text) → writeText(text)` (already present). Task 5 extends `createItem` + adds `cliInstallStatus`/`cliInstall`/`cliUninstall`/`setStatuses`.
- **`src/App.tsx` (current):** already imports `getCurrentWindow` from `@tauri-apps/api/window` (used for `setAlwaysOnTop`), and `createItem` from the client. Holds `settingsOpen`, `promptCollection`, error/confirm dialogs. The "create in selected collection; All → default" rule is in the Cmd+N handler: `const target = sel === ALL_COLLECTION ? undefined : sel`. Task 8 adds `bulkStatusCollection`; Task 9 adds the drag-drop subscription.
- **`src/components/Sidebar.tsx` (current):** the per-collection `ContextMenu.Content` has, in order: `Rename`, `Edit Prompt…`, `Copy Prompt`, `Copy CLI Command`, `Separator`, `Color` (Sub), `Archive/Unarchive`, `Move to Group` (Sub), `Clear` (Sub), `Export Collection` (Sub), `Separator`, `Delete`. The component receives `onEditPrompt`, `onSnapshot`, `onError`, `onRequestConfirm`. Task 8 adds an `onChangeStatuses` prop + a **"Change Statuses…"** item.
- **`src/components/SettingsDialog.tsx` (current):** `Tabs.Root defaultValue="system"` with `system` + `prompt` triggers/contents. Props `{ open, onOpenChange, settings, updateSettings }`. It already imports `getVersion` from `@tauri-apps/api/app` + `storePath` from the client. Task 7 adds a `command` trigger/content + fetches `cliInstallStatus` on tab open.

### Drag-drop event API + permission (verified against the generated schema)

- **API:** `import { getCurrentWindow } from "@tauri-apps/api/window";` then `const unlisten = await getCurrentWindow().onDragDropEvent((event) => { ... })`. The payload is `event.payload` with `payload.type` one of `"enter" | "over" | "drop" | "leave"`; on `"drop"` it carries `payload.paths: string[]` (absolute filesystem paths). Act only on `payload.type === "drop"`. `onDragDropEvent` returns a `Promise<UnlistenFn>`; call it in cleanup. (Verify the exact discriminant spelling against the installed `@tauri-apps/api/window` types; gate = clean tsc + the drop firing at manual launch.)
- **`dragDropEnabled`:** in Tauri v2 the window drag-drop event is **enabled by default** (`dragDropEnabled` defaults to `true`). The current `src-tauri/tauri.conf.json` `app.windows[0]` does **not** set it, so the default applies and no config change is required. (If at manual launch no drop event fires — e.g. a WKWebView intercepts it — set `"dragDropEnabled": true` explicitly on the `main` window; gate for that flag is the user's manual launch, not a unit test.)
- **Permission:** the drag-drop event is delivered through Tauri's core event channel. `core:default` (granted in `src-tauri/capabilities/default.json`) **includes `core:event:default`** (which grants `core:event:allow-listen`/`allow-unlisten`) — confirmed in `src-tauri/gen/schemas/desktop-schema.json` (the only event identifiers are `core:event:*`; there is **no separate `drag-drop` / `dragDrop` permission** in the schema — `grep -io 'drag' src-tauri/gen/schemas/*.json` finds only the unrelated `dragging` window-token). The existing `onStoreChanged` already listens to a custom event under the same `core:default`, so **no capability change is needed** for `onDragDropEvent`. (Verify at manual launch that the listener is not rejected; if a future Tauri requires `core:window:allow-...` for the drag-drop event, add it to `default.json` per the schema's spelling — gate = clean build + the listener firing.)

### `externalBin` / sidecar config shape (Tauri v2)

- **`bundle.externalBin`** is an **array of base paths**, each **relative to `src-tauri/`** and **without** the target-triple suffix or extension: `"bundle": { ..., "externalBin": ["binaries/taskpond"] }`. At build time Tauri requires a file named `src-tauri/binaries/taskpond-<target-triple>` (e.g. `taskpond-aarch64-apple-darwin`) to exist, and copies it next to the main binary in the bundle. The build-copy script (Task 1) produces exactly that file. (Verify the key name + relative-path semantics against `https://schema.tauri.app/config/2`; gate for the actual bundling is the user's `cargo tauri build`.)
- **Host target triple:** `rustc -vV` prints a `host: <triple>` line — parse that for `<target-triple>` (the dev host's triple; a cross/universal build is the user's concern). The script reads it via `child_process.execFileSync("rustc", ["-vV"])`.

### Divergences from the design spec (confirmed against source)

1. **`resolve_taskpond_target` returns `Option<PathBuf>`** (not the spec's bare `PathBuf`): `current_exe()` and `.parent()` are both fallible, so the function is `current_exe().ok()?.parent()?.join("taskpond")` and the command falls back to `PathBuf::from("taskpond")` when `None` (mirroring `Installer::with_defaults`). The testable core is `target_beside(exe: &Path) -> PathBuf` (pure sibling-join), tested without a real `current_exe`.
2. **The whole CLI-install path (`install.rs` + the three commands + the `mod install;` line) is `#[cfg(unix)]`**, because `pond_core::cli_install` is `#![cfg(unix)]` and not re-exported. On a non-unix build these commands simply do not exist (macOS-primary, per design decision 3). The frontend always calls them; on the (unsupported) non-unix build the invoke would reject — acceptable since the target is macOS.
3. **The `Installer` build uses `<home>/.local/bin/taskpond` for the link** (the same default as `with_defaults`) but **overrides the target** with `resolve_taskpond_target()` (the `current_exe` sibling) so dev/bundle both resolve correctly per design decision 2. Home is `directories::BaseDirs::new()` (already a `pond-tauri` dep) → `home_dir()`, fallback `PathBuf::from(".")` (matching `with_defaults`).
4. **No new crates/npm packages.** The clipboard plugin (for the PATH-hint copy button) and `@tauri-apps/api` (for `getCurrentWindow`) are already present from 5A/5B; `cli_install` is already in `pond-core`.
5. **`presentStatuses` ordering** uses the `src/api/types.ts` `TaskStatus` union order (= `TaskStatus::all()` order). No new ordering constant is invented.

---

## File Structure

```
src-tauri/
├─ tauri.conf.json   + bundle.externalBin ["binaries/taskpond"]; beforeDevCommand/beforeBuildCommand run the sidecar build-copy before the existing vite command
├─ binaries/         (gitignored) taskpond-<target-triple> — produced by scripts/build-sidecar.mjs
└─ src/
   ├─ install.rs     (NEW, #[cfg(unix)]) target_beside / resolve_taskpond_target + InstallStatusDto + dto_from(&InstallStatus, path_hint) + #[cfg(test)] tests
   ├─ commands.rs     + cli_install_status / cli_install / cli_uninstall (#[cfg(unix)]); set_statuses; create_item wrapper gains optional title
   ├─ mutations.rs    + set_statuses(store, replacements, collection) -> SnapshotDto; create_item gains optional title arg
   └─ main.rs         + #[cfg(unix)] mod install; register cli_install_status/cli_install/cli_uninstall (#[cfg(unix)]) + set_statuses
scripts/
└─ build-sidecar.mjs (NEW) node: cargo build -p taskpond-cli (profile arg) + read host triple from rustc -vV + copy target/<profile>/taskpond → src-tauri/binaries/taskpond-<triple> (chmod 0o755)

src/
├─ api/
│  ├─ client.ts       + cliInstallStatus/cliInstall/cliUninstall, setStatuses, createItem(collection?, title?)
│  ├─ client.test.ts  + mocked-invoke assertions for cliInstallStatus + setStatuses + createItem(title)
│  └─ types.ts        + InstallStatus interface (camelCase mirror)
├─ state/
│  ├─ bulkStatus.ts      (NEW) pure presentStatuses(snapshot, collection) -> TaskStatus[]
│  └─ bulkStatus.test.ts (NEW) Vitest: distinct, dedup, empty, ordering
├─ components/
│  ├─ SettingsDialog.tsx   + Command tab (install/uninstall UI; fetch cliInstallStatus on open)
│  ├─ BulkStatusDialog.tsx (NEW) present-status → replacement Select grid; Confirm → setStatuses
│  └─ Sidebar.tsx          collection menu: "Change Statuses…" → onChangeStatuses(name)
└─ App.tsx           bulkStatusCollection state + host BulkStatusDialog; drag-drop subscription → createItem(targetCollection, basename) per dropped file
.gitignore           + /src-tauri/binaries/
```

Each unit stays focused: `install.rs` is pure (sibling-join + DTO mapping, tested without Tauri `State`); `mutations.rs` is `&TaskStore`-testable; `commands.rs` wrappers contain no logic beyond `State` access → string error; `api/`+`state/` are the only `invoke`/pure-logic sites; components render and dispatch; the sidecar script is build tooling.

---

## Task 1: Sidecar packaging (externalBin + build-copy script)

Configure `taskpond` as a Tauri `externalBin` sidecar and add a Node build-copy script that builds the CLI and stages it where Tauri expects (`src-tauri/binaries/taskpond-<host-triple>`). Wire the script into Tauri's dev/build hooks and gitignore the staged artifact. Foundation only — no unit tests; verification per the **Task 1 scope** above (script runs + produces an executable + Rust build stays clean + conf parses), **not** a full `cargo tauri dev`/`build`.

**Files:**
- Create: `scripts/build-sidecar.mjs`
- Modify: `src-tauri/tauri.conf.json`, `.gitignore`

- [ ] **Step 1: Write `scripts/build-sidecar.mjs`**

Create `scripts/build-sidecar.mjs`:

```js
// Build the `taskpond` CLI and stage it as a Tauri sidecar:
//   target/<profile>/taskpond  ->  src-tauri/binaries/taskpond-<host-triple>
// Usage: node scripts/build-sidecar.mjs [debug|release]   (default: debug)
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, chmodSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const profile = process.argv[2] === "release" ? "release" : "debug";

// 1. Build the CLI crate.
const buildArgs = ["build", "-p", "taskpond-cli"];
if (profile === "release") buildArgs.push("--release");
execFileSync("cargo", buildArgs, { cwd: repoRoot, stdio: "inherit" });

// 2. Read the host target triple from `rustc -vV` (the "host: <triple>" line).
const rustcOut = execFileSync("rustc", ["-vV"], { cwd: repoRoot, encoding: "utf8" });
const hostLine = rustcOut.split("\n").find((l) => l.startsWith("host:"));
if (!hostLine) {
  throw new Error("could not determine host target triple from `rustc -vV`");
}
const triple = hostLine.slice("host:".length).trim();

// 3. Copy target/<profile>/taskpond -> src-tauri/binaries/taskpond-<triple> (executable).
const src = join(repoRoot, "target", profile, "taskpond");
const destDir = join(repoRoot, "src-tauri", "binaries");
const dest = join(destDir, `taskpond-${triple}`);
mkdirSync(destDir, { recursive: true });
copyFileSync(src, dest);
chmodSync(dest, 0o755);

console.log(`sidecar staged: ${dest}`);
```

(If the CLI crate's package name is not `taskpond-cli`, adjust the `-p` arg to the actual package name found via `cargo metadata` / the CLI crate's `Cargo.toml` `[package] name`; the produced **binary** is `taskpond` either way — adjust the `src` filename only if the binary name differs. Gate = the script prints `sidecar staged: …` and the file exists.)

- [ ] **Step 2: Add `externalBin` + wire the build-copy into the hooks in `tauri.conf.json`**

Edit `src-tauri/tauri.conf.json`. In `build`, prepend the build-copy step before each existing vite command, and add `externalBin` to `bundle`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Pond",
  "version": "0.1.0",
  "identifier": "dev.kymok.pond",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "node scripts/build-sidecar.mjs debug && npm run dev",
    "beforeBuildCommand": "node scripts/build-sidecar.mjs release && npm run build"
  },
  "app": {
    "windows": [{ "label": "main", "title": "Pond", "width": 900, "height": 600, "minWidth": 480, "minHeight": 320 }],
    "security": { "csp": null }
  },
  "bundle": { "active": true, "targets": "all", "icon": ["icons/icon.png"], "externalBin": ["binaries/taskpond"] }
}
```

(`externalBin` is an array of `src-tauri/`-relative base paths without the triple/extension; Tauri resolves each to `binaries/taskpond-<target-triple>`. Verify the key name + path semantics against `https://schema.tauri.app/config/2`; gate for actual bundling is the user's `cargo tauri build`.)

- [ ] **Step 3: Gitignore the staged binaries**

Edit `.gitignore`. After the `/src-tauri/gen/` line, add:

```gitignore
/src-tauri/binaries/
```

- [ ] **Step 4: Verify (Task 1 scope — NOT a full tauri dev/build)**

```bash
node scripts/build-sidecar.mjs debug
ls -l src-tauri/binaries/
cargo build -p pond-tauri
node -e "JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json','utf8')); console.log('conf ok')"
```
Expected: the script prints `sidecar staged: …/src-tauri/binaries/taskpond-<triple>`; `ls -l` shows that file with an executable bit (`-rwxr-xr-x`); `cargo build -p pond-tauri` is clean; the node one-liner prints `conf ok`. (`taskpond-<triple>` is gitignored — confirm `git status` does **not** show it.)

- [ ] **Step 5: Commit**

```bash
git add scripts/build-sidecar.mjs src-tauri/tauri.conf.json .gitignore
git commit -m "feat(tauri): package taskpond as a sidecar (externalBin + build-copy script)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 2: `install.rs` + cli_install commands (`cfg(unix)`)

Add a new `#[cfg(unix)]` module with the testable target-resolution + DTO mapping, and the three CLI-install commands that build an `Installer` (default link/record, `current_exe`-sibling target) and map its `InstallStatus` to a camelCase DTO. Port of Swift `AppModel.installCLI`/`uninstallCLI`/`refreshCLIStatus` + `CLIInstallStatus`.

**Files:**
- Create: `src-tauri/src/install.rs`
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Write `install.rs` with the implementation + failing tests**

Create `src-tauri/src/install.rs`:

```rust
//! CLI-install IPC support (unix-only; mirrors pond_core::cli_install, which is #![cfg(unix)]).
//! `target_beside`/`resolve_taskpond_target` pick the install target (the binary beside the
//! app executable); `InstallStatusDto` + `dto_from` map pond-core's `InstallStatus` to a
//! camelCase wire DTO (adding the derived `canInstall` and the `Installer`-level `pathHint`).
use pond_core::cli_install::InstallStatus;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// The install target sitting beside a given executable path: `<exe-dir>/taskpond`.
/// Pure (no `current_exe`) so it is unit-testable.
pub fn target_beside(exe: &Path) -> PathBuf {
    exe.parent().unwrap_or_else(|| Path::new("")).join("taskpond")
}

/// The real install target: `taskpond` next to the current executable. `None` when the
/// current exe / its parent cannot be resolved (caller falls back to a bare "taskpond").
pub fn resolve_taskpond_target() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let parent = exe.parent()?;
    Some(parent.join("taskpond"))
}

/// camelCase wire mirror of `InstallStatus` + the derived `canInstall` + `pathHint`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallStatusDto {
    pub link_path: String,
    pub target_path: String,
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_description: Option<String>,
    pub install_directory_is_in_path: bool,
    pub can_uninstall: bool,
    pub can_install: bool,
    pub path_hint: String,
}

/// Build the DTO from an `InstallStatus` plus the `Installer`-level `path_hint`
/// (`path_hint` is NOT a field of `InstallStatus`, and `can_install` is a method).
pub fn dto_from(status: &InstallStatus, path_hint: String) -> InstallStatusDto {
    InstallStatusDto {
        link_path: status.link_path.to_string_lossy().to_string(),
        target_path: status.target_path.to_string_lossy().to_string(),
        installed: status.installed,
        conflict_description: status.conflict_description.clone(),
        install_directory_is_in_path: status.install_directory_is_in_path,
        can_uninstall: status.can_uninstall,
        can_install: status.can_install(),
        path_hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pond_core::cli_install::Installer;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use tempfile::tempdir;

    fn make_executable(path: &Path) {
        fs::write(path, b"#!/bin/sh\n").unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn target_beside_joins_taskpond_to_parent() {
        assert_eq!(
            target_beside(Path::new("/Applications/Pond.app/Contents/MacOS/Pond")),
            PathBuf::from("/Applications/Pond.app/Contents/MacOS/taskpond")
        );
        // No parent → just "taskpond".
        assert_eq!(target_beside(Path::new("Pond")), PathBuf::from("taskpond"));
    }

    #[test]
    fn dto_from_round_trips_fields_and_derives_can_install() {
        // A not-installed, no-conflict status → can_install == true (per InstallStatus::can_install).
        let status = InstallStatus {
            link_path: PathBuf::from("/home/u/.local/bin/taskpond"),
            target_path: PathBuf::from("/app/taskpond"),
            installed: false,
            conflict_description: None,
            install_directory_is_in_path: false,
            can_uninstall: false,
        };
        let dto = dto_from(&status, "EXPORT".to_string());
        assert_eq!(dto.link_path, "/home/u/.local/bin/taskpond");
        assert_eq!(dto.target_path, "/app/taskpond");
        assert!(!dto.installed);
        assert_eq!(dto.conflict_description, None);
        assert!(!dto.install_directory_is_in_path);
        assert!(!dto.can_uninstall);
        assert!(dto.can_install); // derived from the method
        assert_eq!(dto.path_hint, "EXPORT");
    }

    #[test]
    fn dto_from_installed_status_cannot_install() {
        let status = InstallStatus {
            link_path: PathBuf::from("/l"),
            target_path: PathBuf::from("/t"),
            installed: true,
            conflict_description: None,
            install_directory_is_in_path: true,
            can_uninstall: true,
        };
        let dto = dto_from(&status, String::new());
        assert!(dto.installed);
        assert!(!dto.can_install); // installed → cannot install
        assert!(dto.can_uninstall);
    }

    #[test]
    fn dto_installed_flag_flips_across_install_uninstall() {
        // Build an Installer on tempdir paths with an executable target, then drive
        // status -> install -> status -> uninstall -> status through the DTO mapping.
        let dir = tempdir().unwrap();
        let target = dir.path().join("taskpond-bin");
        make_executable(&target);
        let link = dir.path().join("bin/taskpond");
        let record = dir.path().join("cli-install.json");
        let installer = Installer::new(link, target, record);

        let before = dto_from(&installer.status(), installer.path_hint());
        assert!(!before.installed);

        installer.install().unwrap();
        let after_install = dto_from(&installer.status(), installer.path_hint());
        assert!(after_install.installed);
        assert!(!after_install.can_install);

        installer.uninstall().unwrap();
        let after_uninstall = dto_from(&installer.status(), installer.path_hint());
        assert!(!after_uninstall.installed);
    }
}
```

- [ ] **Step 2: Declare the module (`cfg(unix)`) in `main.rs`**

Edit `src-tauri/src/main.rs`. Add the gated module declaration after `mod dto;` (alphabetical):

```rust
mod commands;
mod dto;
#[cfg(unix)]
mod install;
mod mutations;
mod prompt;
mod settings;
mod watcher;
```

- [ ] **Step 3: Run the `install` tests**

```bash
cargo test -p pond-tauri install
```
Expected: the 4 `install::tests` pass. (If `pond_core::cli_install::InstallStatus`'s field set differs from the Verified-facts block, the struct literal in the tests won't compile — re-check `crates/pond-core/src/cli_install.rs` and match it exactly.)

- [ ] **Step 4: Add the three commands (`cfg(unix)`) to `commands.rs`**

Edit `src-tauri/src/commands.rs`. The top `use` block already has `Result`, `TaskStore`, `State`. Append the three commands before the `#[cfg(test)]` module:

```rust
#[cfg(unix)]
fn build_installer() -> pond_core::cli_install::Installer {
    use std::path::PathBuf;
    let home = directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let link = home.join(".local/bin/taskpond");
    let target = crate::install::resolve_taskpond_target().unwrap_or_else(|| PathBuf::from("taskpond"));
    let record = pond_core::paths::data_directory().join("cli-install.json");
    pond_core::cli_install::Installer::new(link, target, record)
}

#[cfg(unix)]
#[tauri::command]
pub fn cli_install_status() -> std::result::Result<crate::install::InstallStatusDto, String> {
    let installer = build_installer();
    Ok(crate::install::dto_from(&installer.status(), installer.path_hint()))
}

#[cfg(unix)]
#[tauri::command]
pub fn cli_install() -> std::result::Result<crate::install::InstallStatusDto, String> {
    let installer = build_installer();
    installer.install().map_err(|e| e.to_string())?;
    Ok(crate::install::dto_from(&installer.status(), installer.path_hint()))
}

#[cfg(unix)]
#[tauri::command]
pub fn cli_uninstall() -> std::result::Result<crate::install::InstallStatusDto, String> {
    let installer = build_installer();
    installer.uninstall().map_err(|e| e.to_string())?;
    Ok(crate::install::dto_from(&installer.status(), installer.path_hint()))
}
```

(Mirrors Swift `installCLI`/`uninstallCLI`: do the action, then re-read status; an error maps to a string. The `build_installer` helper is `#[cfg(unix)]` because `Installer` is unix-only. `directories::BaseDirs` is already a dep.)

- [ ] **Step 5: Register the commands (`cfg(unix)`) in `main.rs`**

Edit `src-tauri/src/main.rs`. `tauri::generate_handler!` does not accept `#[cfg]` lines inside the macro on every Tauri version, so gate the registration by listing the commands conditionally. Replace the single `generate_handler!` invocation with a `cfg`-split that appends the three unix commands only on unix. The simplest robust form that compiles everywhere: keep the existing list and, on unix, add the three entries inline. Use this exact shape (verify it compiles; if the macro rejects inline `#[cfg]`, fall back to the two-arm `if cfg!(...)` builder split noted below):

```rust
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::create_item,
            commands::update_item,
            commands::set_status,
            commands::move_item,
            commands::delete_item,
            commands::delete_items,
            commands::add_note,
            commands::update_note,
            commands::delete_note,
            commands::merge_item,
            commands::split_item,
            commands::create_collection,
            commands::rename_collection,
            commands::set_collection_color,
            commands::set_collection_archived,
            commands::move_collection,
            commands::clear_items,
            commands::delete_collection,
            commands::create_group,
            commands::rename_group,
            commands::delete_group,
            commands::get_settings,
            commands::set_settings,
            commands::store_path,
            commands::set_collection_prompt,
            commands::collection_prompt_text,
            commands::collection_cli_command,
            commands::export_collection,
            #[cfg(unix)]
            commands::cli_install_status,
            #[cfg(unix)]
            commands::cli_install,
            #[cfg(unix)]
            commands::cli_uninstall,
        ])
```

If `cargo build` reports that `generate_handler!` does not accept attributes on its items (older macro), instead split the builder: define `let builder = tauri::Builder::default().plugin(...).plugin(...);` then on unix `let builder = builder.invoke_handler(tauri::generate_handler![/* full list incl. the 3 cli commands */]);` and on non-unix `let builder = builder.invoke_handler(tauri::generate_handler![/* list without the 3 */]);` via `#[cfg(unix)]` / `#[cfg(not(unix))]` blocks, then `builder.setup(...).run(...)`. Gate = `cargo build -p pond-tauri` clean on the (unix) dev host.

- [ ] **Step 6: Test + gate**

```bash
cargo test -p pond-tauri
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: all `pond-tauri` tests pass (incl. the 4 `install::tests`); fmt/clippy clean.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/install.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(tauri): cli-install commands + InstallStatusDto (unix)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 3: `set_statuses` command (bulk-status remap)

Add the per-collection bulk-status remap: a `mutations::set_statuses` that delegates to `TaskStore::set_statuses` (ids empty, scoped to a collection) and rebuilds the snapshot, plus the command wrapper. Port of Swift `AppModel.confirmBulkStatusChange` (the `.collection` branch). Includes a test proving the wire `HashMap<TaskStatus,TaskStatus>` deserializes from a JSON object.

**Files:**
- Modify: `src-tauri/src/mutations.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Add `mutations::set_statuses` with a failing test**

Edit `src-tauri/src/mutations.rs`. Add `use std::collections::HashMap;` to the top `use` block. Append the mutation after `create_item` (above the `#[cfg(test)]` module):

```rust
/// Remap statuses within a single collection: every item whose current status is a key
/// in `replacements` is set to the mapped value (no-op pairs are ignored by pond-core).
/// `ids` empty + `Some(collection)` scopes it to the whole collection. Returns the snapshot.
pub fn set_statuses(
    store: &TaskStore,
    replacements: &HashMap<TaskStatus, TaskStatus>,
    collection: &str,
) -> Result<SnapshotDto> {
    store.set_statuses(replacements, &[], Some(collection))?;
    build_snapshot(store)
}
```

Add `use std::collections::HashMap;` to the existing `mod tests` top `use` block, then add this test inside it:

```rust
    #[test]
    fn set_statuses_remaps_within_collection() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("A", "Work", None, false, TaskStatus::Ready).unwrap();
        store.add("B", "Work", None, false, TaskStatus::Ready).unwrap();
        store.add("C", "Work", None, false, TaskStatus::InProgress).unwrap();
        // A different collection that must be untouched.
        store.add("X", "Home", None, false, TaskStatus::Ready).unwrap();

        let mut replacements = HashMap::new();
        replacements.insert(TaskStatus::Ready, TaskStatus::Completed);
        let snap = set_statuses(&store, &replacements, "Work").unwrap();

        // Work: the two Ready items are now Completed; the InProgress is unchanged.
        let work: Vec<&TaskItem> = snap.items.iter().filter(|i| i.collection == "Work").collect();
        assert_eq!(work.iter().filter(|i| i.status == TaskStatus::Completed).count(), 2);
        assert_eq!(work.iter().filter(|i| i.status == TaskStatus::InProgress).count(), 1);
        assert_eq!(work.iter().filter(|i| i.status == TaskStatus::Ready).count(), 0);

        // Home is untouched (still Ready).
        let home: Vec<&TaskItem> = snap.items.iter().filter(|i| i.collection == "Home").collect();
        assert_eq!(home.len(), 1);
        assert_eq!(home[0].status, TaskStatus::Ready);
    }

    #[test]
    fn replacements_map_deserializes_from_json_object() {
        // The wire shape is a JS object of status-string -> status-string.
        let map: HashMap<TaskStatus, TaskStatus> =
            serde_json::from_str(r#"{"ready":"completed","in-progress":"on-hold"}"#).unwrap();
        assert_eq!(map.get(&TaskStatus::Ready), Some(&TaskStatus::Completed));
        assert_eq!(map.get(&TaskStatus::InProgress), Some(&TaskStatus::OnHold));
    }
```

(`serde_json` is a `pond-tauri` dependency, so `serde_json::from_str` is available in the test without a new `use`.)

Run:

```bash
cargo test -p pond-tauri set_statuses
```
Expected: **fail** to compile (`mutations::set_statuses` does not exist) → pass once Step 1's code is in.

- [ ] **Step 2: Add the `set_statuses` command to `commands.rs`**

Edit `src-tauri/src/commands.rs`. The top `use` block already imports `HashMap`, `TaskStatus`, `TaskStore`, `SnapshotDto`, `mutations`, `State`. Append the command before `#[cfg(test)]`:

```rust
#[tauri::command]
pub fn set_statuses(
    store: State<TaskStore>,
    replacements: HashMap<TaskStatus, TaskStatus>,
    collection: String,
) -> std::result::Result<SnapshotDto, String> {
    mutations::set_statuses(&store, &replacements, &collection).map_err(|e| e.to_string())
}
```

- [ ] **Step 3: Register `set_statuses` in `main.rs`**

Edit `src-tauri/src/main.rs`. In `generate_handler!`, append after `commands::export_collection,` (before the `#[cfg(unix)]` cli-install entries):

```rust
            commands::set_statuses,
```

- [ ] **Step 4: Test + gate**

```bash
cargo test -p pond-tauri
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: all tests pass (incl. the two new `mutations` tests); fmt/clippy clean.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/mutations.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(tauri): set_statuses command (per-collection bulk remap)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 4: `create_item` title extension (file-drop support)

Extend `create_item` with an optional `title` so a dropped file can create a Draft titled with its filename, while Cmd+N/New Task (no title) keeps creating an empty Draft. Port of Swift `createTaskFromDroppedFile` → `createTask(title: filename, status: .draft)`.

**Files:**
- Modify: `src-tauri/src/mutations.rs`, `src-tauri/src/commands.rs`

- [ ] **Step 1: Extend `mutations::create_item` with a failing test**

Edit `src-tauri/src/mutations.rs`. Replace the existing `create_item` with the title-aware version:

```rust
/// Create a new Draft. `collection` is the target collection api-name (`None`/empty →
/// the default collection); `title` is the Draft's title (`None` → empty, for Cmd+N /
/// New Task; `Some(name)` → a file-drop Draft titled with the filename).
pub fn create_item(
    store: &TaskStore,
    collection: Option<&str>,
    title: Option<&str>,
) -> Result<SnapshotDto> {
    let target = collection
        .filter(|c| !c.is_empty())
        .unwrap_or(DEFAULT_COLLECTION);
    store.add(title.unwrap_or(""), target, None, true, TaskStatus::Draft)?;
    build_snapshot(store)
}
```

Add this test inside the existing `mod tests`:

```rust
    #[test]
    fn create_item_title_none_is_empty_draft() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let snap = create_item(&store, Some("Work"), None).unwrap();
        let item = snap.items.iter().find(|i| i.collection == "Work").unwrap();
        assert_eq!(item.title, "");
        assert_eq!(item.status, TaskStatus::Draft);
    }

    #[test]
    fn create_item_title_some_creates_titled_draft() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let snap = create_item(&store, Some("Work"), Some("foo.txt")).unwrap();
        let item = snap.items.iter().find(|i| i.title == "foo.txt").unwrap();
        assert_eq!(item.collection, "Work");
        assert_eq!(item.status, TaskStatus::Draft);
    }
```

Run:

```bash
cargo test -p pond-tauri create_item
```
Expected: **fail** to compile (the call sites / the command wrapper still pass the old 2-arg form) → the `mutations` tests compile against the new signature; the command wrapper is fixed in Step 2.

- [ ] **Step 2: Update the `create_item` command wrapper**

Edit `src-tauri/src/commands.rs`. Replace the existing `create_item` command with the title-aware version:

```rust
#[tauri::command]
pub fn create_item(
    store: State<TaskStore>,
    collection: Option<String>,
    title: Option<String>,
) -> std::result::Result<SnapshotDto, String> {
    mutations::create_item(&store, collection.as_deref(), title.as_deref()).map_err(|e| e.to_string())
}
```

(No registration change — `create_item` is already in `generate_handler!`. There are no other internal callers of `mutations::create_item`.)

- [ ] **Step 3: Test + gate**

```bash
cargo test -p pond-tauri
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: all tests pass (incl. the two new `create_item` cases); fmt/clippy clean.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/mutations.rs src-tauri/src/commands.rs
git commit -m "feat(tauri): create_item optional title (file-drop support)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 5: frontend client wrappers + types

Add the `InstallStatus` DTO type, the three cli-install wrappers, the `setStatuses` wrapper, and extend `createItem` with an optional `title`; cover the new wrappers with mocked-invoke tests.

**Files:**
- Modify: `src/api/types.ts`, `src/api/client.ts`, `src/api/client.test.ts`

- [ ] **Step 1: Add the `InstallStatus` interface to `types.ts`**

Edit `src/api/types.ts`. Append (after the `Settings` interface):

```ts
export interface InstallStatus {
  linkPath: string;
  targetPath: string;
  installed: boolean;
  conflictDescription?: string;
  installDirectoryIsInPath: boolean;
  canUninstall: boolean;
  canInstall: boolean;
  pathHint: string;
}
```

- [ ] **Step 2: Failing tests for the new client wrappers**

Append to `src/api/client.test.ts` (inside the existing `describe`):

```ts
  it("cliInstallStatus invokes cli_install_status", async () => {
    invokeMock.mockResolvedValue({
      linkPath: "/l", targetPath: "/t", installed: false,
      installDirectoryIsInPath: false, canUninstall: false, canInstall: true, pathHint: "EXPORT",
    });
    const s = await cliInstallStatus();
    expect(invokeMock).toHaveBeenCalledWith("cli_install_status");
    expect(s.canInstall).toBe(true);
  });

  it("setStatuses invokes set_statuses with replacements + collection", async () => {
    invokeMock.mockResolvedValue({ items: [], collections: [], groups: [] });
    await setStatuses({ ready: "completed" }, "Work");
    expect(invokeMock).toHaveBeenCalledWith("set_statuses", {
      replacements: { ready: "completed" },
      collection: "Work",
    });
  });

  it("createItem passes collection + title", async () => {
    invokeMock.mockResolvedValue({ items: [], collections: [], groups: [] });
    await createItem("Work", "foo.txt");
    expect(invokeMock).toHaveBeenCalledWith("create_item", { collection: "Work", title: "foo.txt" });
  });

  it("createItem with no args passes nulls", async () => {
    invokeMock.mockResolvedValue({ items: [], collections: [], groups: [] });
    await createItem();
    expect(invokeMock).toHaveBeenCalledWith("create_item", { collection: null, title: null });
  });
```

Extend the top import in `client.test.ts` to include the new/used wrappers (merge into the existing import list — `createItem` is likely already imported; add only what is missing):

```ts
import {
  createItem,
  cliInstallStatus,
  setStatuses,
} from "./client";
```

Run:

```bash
npx vitest run src/api/client.test.ts
```
Expected: **fail** (`cliInstallStatus`/`setStatuses` not exported; `createItem` does not yet send `title`).

- [ ] **Step 3: Add/extend the wrappers in `client.ts`**

Edit `src/api/client.ts`. Add `InstallStatus` to the type import:

```ts
import type { CollectionColor, InstallStatus, Settings, Snapshot, TaskItem, TaskStatus } from "./types";
```

Replace the existing `createItem` with the title-aware version (still in the `// --- Items ---` section):

```ts
export function createItem(collection?: string, title?: string): Promise<Snapshot> {
  return invoke<Snapshot>("create_item", { collection: collection ?? null, title: title ?? null });
}
```

Append a new section at the end of the file:

```ts
// --- CLI install / bulk status ---
export function cliInstallStatus(): Promise<InstallStatus> {
  return invoke<InstallStatus>("cli_install_status");
}

export function cliInstall(): Promise<InstallStatus> {
  return invoke<InstallStatus>("cli_install");
}

export function cliUninstall(): Promise<InstallStatus> {
  return invoke<InstallStatus>("cli_uninstall");
}

export function setStatuses(
  replacements: Record<string, string>,
  collection: string,
): Promise<Snapshot> {
  return invoke<Snapshot>("set_statuses", { replacements, collection });
}
```

- [ ] **Step 4: Run tests (passing) + gate**

```bash
npx vitest run src/api/client.test.ts
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: green/clean. (If any existing `createItem(undefined)` call site needs no change — the second arg is optional — confirm `tsc` stays clean.)

- [ ] **Step 5: Commit**

```bash
git add src/api/types.ts src/api/client.ts src/api/client.test.ts
git commit -m "feat(ui): cli-install + setStatuses client wrappers, createItem title

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 6: `state/bulkStatus.ts` `presentStatuses` + tests

A pure helper returning the distinct statuses present among a collection's items, in the canonical status order (the `TaskStatus` union order). Drives the bulk-status dialog rows.

**Files:**
- Create: `src/state/bulkStatus.ts`, `src/state/bulkStatus.test.ts`

- [ ] **Step 1: Failing tests**

Create `src/state/bulkStatus.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import type { Snapshot, TaskItem, TaskStatus } from "../api/types";
import { presentStatuses } from "./bulkStatus";

function item(collection: string, status: TaskStatus): TaskItem {
  return {
    id: `${collection}-${status}-${Math.random()}`,
    version: "1",
    title: "t",
    collection,
    status,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };
}

function snap(items: TaskItem[]): Snapshot {
  return { items, collections: [], groups: [] };
}

describe("presentStatuses", () => {
  it("returns the distinct statuses in a collection, deduped", () => {
    const s = snap([
      item("Work", "ready"),
      item("Work", "ready"),
      item("Work", "in-progress"),
      item("Home", "completed"), // different collection — ignored
    ]);
    expect(presentStatuses(s, "Work")).toEqual(["ready", "in-progress"]);
  });

  it("orders by the canonical TaskStatus order regardless of item order", () => {
    const s = snap([
      item("Work", "aborted"),
      item("Work", "draft"),
      item("Work", "completed"),
      item("Work", "ready"),
    ]);
    expect(presentStatuses(s, "Work")).toEqual(["draft", "ready", "completed", "aborted"]);
  });

  it("returns an empty array for a collection with no items", () => {
    expect(presentStatuses(snap([]), "Work")).toEqual([]);
    expect(presentStatuses(snap([item("Home", "ready")]), "Work")).toEqual([]);
  });
});
```

Run:

```bash
npx vitest run src/state/bulkStatus.test.ts
```
Expected: **fail** (`./bulkStatus` missing).

- [ ] **Step 2: Implement `state/bulkStatus.ts`**

Create `src/state/bulkStatus.ts`:

```ts
import type { Snapshot, TaskStatus } from "../api/types";

// Canonical status order = the pond-core TaskStatus::all() order (and the types.ts union order).
const STATUS_ORDER: TaskStatus[] = [
  "draft",
  "ready",
  "in-progress",
  "completed",
  "on-hold",
  "rejected",
  "aborted",
];

/** The distinct statuses among `collection`'s items, in canonical status order. */
export function presentStatuses(snapshot: Snapshot, collection: string): TaskStatus[] {
  const present = new Set<TaskStatus>();
  for (const item of snapshot.items) {
    if (item.collection === collection) present.add(item.status);
  }
  return STATUS_ORDER.filter((s) => present.has(s));
}
```

- [ ] **Step 3: Run tests (passing) + gate**

```bash
npx vitest run src/state/bulkStatus.test.ts
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: green/clean.

- [ ] **Step 4: Commit**

```bash
git add src/state/bulkStatus.ts src/state/bulkStatus.test.ts
git commit -m "feat(ui): presentStatuses helper for the bulk-status dialog

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 7: Settings Command tab

Add a **Command** tab to `SettingsDialog` (port of Swift `commandSection` + `statusText`): Link, Status (installed/conflict/not-installed), Add-to-PATH hint + copy (when not in PATH), and Install/Reinstall + Uninstall buttons. Fetch `cliInstallStatus()` when the Command tab opens and refresh from the DTO returned by install/uninstall.

> **Radix 3.3.0 latitude:** uses `Tabs.Trigger`/`Tabs.Content` (already in this file), `Flex`, `Text`, `Code`, `Button`, and `Link` (for the selectable link path) + `IconButton`/`Button` with `@radix-ui/react-icons`'s `CopyIcon` for the PATH copy. If 3.3.0's `Code`/`Link`/`IconButton` prop surface differs, adjust to the installed API; gate is clean tsc+build.

**Files:**
- Modify: `src/components/SettingsDialog.tsx`

- [ ] **Step 1: Imports, state, and the status fetch**

Edit `src/components/SettingsDialog.tsx`. Extend the imports to add the cli-install wrappers, the clipboard helper, the `InstallStatus` type, and the Radix/icon parts used by the tab:

```ts
import { useEffect, useState } from "react";
import { Button, Code, Dialog, Flex, IconButton, Link, Tabs, Text, TextArea } from "@radix-ui/themes";
import { CopyIcon } from "@radix-ui/react-icons";
import { getVersion } from "@tauri-apps/api/app";
import { cliInstall, cliInstallStatus, cliUninstall, storePath } from "../api/client";
import { copyText } from "../lib/clipboard";
import type { InstallStatus, Settings } from "../api/types";
```

Add the install-status state next to the existing `version`/`path`/`promptDraft` state:

```ts
  const [installStatus, setInstallStatus] = useState<InstallStatus | null>(null);
  const [installError, setInstallError] = useState<string | null>(null);
```

Add an effect that fetches the status when the dialog is open (so switching to the Command tab shows fresh data; cheap to re-fetch). Place it after the existing version/path effect:

```ts
  // Fetch CLI-install status whenever the dialog opens (the Command tab reads it).
  useEffect(() => {
    if (!open) return;
    cliInstallStatus()
      .then((s) => { setInstallStatus(s); setInstallError(null); })
      .catch((e) => setInstallError(String(e)));
  }, [open]);
```

Add the install/uninstall handlers inside the component (before `return`):

```ts
  const runInstall = () => {
    cliInstall()
      .then((s) => { setInstallStatus(s); setInstallError(null); })
      .catch((e) => setInstallError(String(e)));
  };
  const runUninstall = () => {
    cliUninstall()
      .then((s) => { setInstallStatus(s); setInstallError(null); })
      .catch((e) => setInstallError(String(e)));
  };
  const statusText = (s: InstallStatus): string =>
    s.installed ? "Installed" : (s.conflictDescription ?? "Not installed");
```

- [ ] **Step 2: Add the Command `Tabs.Trigger` + `Tabs.Content`**

Edit the `Tabs.List` to add the Command trigger after the others:

```tsx
          <Tabs.List>
            <Tabs.Trigger value="system">System Information</Tabs.Trigger>
            <Tabs.Trigger value="prompt">Prompt</Tabs.Trigger>
            <Tabs.Trigger value="command">Command</Tabs.Trigger>
          </Tabs.List>
```

Add the Command `Tabs.Content` after the `prompt` content:

```tsx
          <Tabs.Content value="command">
            <Flex direction="column" gap="3" mt="3">
              {installStatus ? (
                <>
                  <Flex direction="column" gap="1">
                    <Text size="2" color="gray">Link</Text>
                    <Link size="1" style={{ wordBreak: "break-all" }}>{installStatus.linkPath}</Link>
                  </Flex>
                  <Flex direction="column" gap="1">
                    <Text size="2" color="gray">Status</Text>
                    <Text size="2" color={installStatus.installed ? "green" : "gray"}>
                      {statusText(installStatus)}
                    </Text>
                  </Flex>
                  {!installStatus.installDirectoryIsInPath && (
                    <Flex direction="column" gap="1">
                      <Text size="2" color="gray">Add to PATH</Text>
                      <Flex align="center" gap="2">
                        <Code size="1" style={{ wordBreak: "break-all" }}>{installStatus.pathHint}</Code>
                        <IconButton
                          size="1"
                          variant="soft"
                          aria-label="Copy PATH command"
                          onClick={() => copyText(installStatus.pathHint).catch((e) => setInstallError(String(e)))}
                        >
                          <CopyIcon />
                        </IconButton>
                      </Flex>
                    </Flex>
                  )}
                  {installError && <Text size="1" color="red">{installError}</Text>}
                  <Flex gap="2" justify="end">
                    <Button
                      color="red"
                      variant="soft"
                      disabled={!installStatus.canUninstall}
                      onClick={runUninstall}
                    >
                      Uninstall
                    </Button>
                    <Button
                      disabled={!installStatus.installed && !installStatus.canInstall}
                      onClick={runInstall}
                    >
                      {installStatus.installed ? "Reinstall" : "Install"}
                    </Button>
                  </Flex>
                </>
              ) : (
                <Text size="2" color={installError ? "red" : "gray"}>
                  {installError ?? "Loading…"}
                </Text>
              )}
            </Flex>
          </Tabs.Content>
```

(Parity with Swift `commandSection`: Link is selectable; Status is green when installed else the conflict text or "Not installed"; the PATH hint + copy button only show when `!installDirectoryIsInPath`; the install button is "Reinstall" when installed, disabled when `!installed && !canInstall`; Uninstall is disabled when `!canUninstall`. Errors surface inline within the tab — `SettingsDialog` is not threaded the app-level `onError`. If the installed Radix `Code`/`Link`/`IconButton` props differ in 3.3.0, adjust; gate = clean tsc+build.)

- [ ] **Step 3: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green. (Report any `Code`/`Link`/`IconButton`/`Tabs` 3.3.0 adjustment.)

- [ ] **Step 4: Commit**

```bash
git add src/components/SettingsDialog.tsx
git commit -m "feat(ui): Settings Command tab (CLI install/uninstall)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 8: `BulkStatusDialog` + "Change Statuses…"

A new dialog rendering one row per present status (`<status> → <Select replacement>`, default = unchanged); Confirm builds the changed-only `replacements` map and calls `setStatuses(map, collection)`. App holds `bulkStatusCollection`; the Sidebar collection menu opens it via **"Change Statuses…"**. Port of Swift `BulkStatusChangeSheet` (the present-status → replacement grid, `.collection` scope).

> **Radix 3.3.0 latitude:** uses `Dialog` (controlled `open`/`onOpenChange`), `Select.Root`/`Trigger`/`Content`/`Item`, `Flex`, `Text`, `Button`. If 3.3.0's `Select`/`Dialog` prop shape differs, adjust to the installed API; gate is clean tsc+build.

**Files:**
- Create: `src/components/BulkStatusDialog.tsx`
- Modify: `src/App.tsx`, `src/components/Sidebar.tsx`

- [ ] **Step 1: Implement `BulkStatusDialog.tsx`**

Create `src/components/BulkStatusDialog.tsx`:

```tsx
import { useEffect, useState } from "react";
import { Button, Dialog, Flex, Select, Text } from "@radix-ui/themes";
import type { Snapshot, TaskStatus } from "../api/types";
import { setStatuses } from "../api/client";
import { presentStatuses } from "../state/bulkStatus";

const STATUS_LABELS: Record<TaskStatus, string> = {
  draft: "Draft",
  ready: "Ready",
  "in-progress": "In Progress",
  completed: "Completed",
  "on-hold": "On Hold",
  rejected: "Rejected",
  aborted: "Aborted",
};

const ALL_STATUSES: TaskStatus[] = [
  "draft", "ready", "in-progress", "completed", "on-hold", "rejected", "aborted",
];

export interface BulkStatusDialogProps {
  /** The collection whose statuses are being remapped; null = closed. */
  collection: string | null;
  snapshot: Snapshot;
  onClose: () => void;
  onSnapshot: (snap: Snapshot) => void;
  onError: (msg: string) => void;
}

export function BulkStatusDialog({
  collection,
  snapshot,
  onClose,
  onSnapshot,
  onError,
}: BulkStatusDialogProps) {
  const rows = collection === null ? [] : presentStatuses(snapshot, collection);
  // Selection per present status; default = unchanged (the same status).
  const [selections, setSelections] = useState<Record<string, TaskStatus>>({});

  // Reset selections to "unchanged" whenever the dialog (re)opens for a collection.
  useEffect(() => {
    if (collection === null) return;
    const init: Record<string, TaskStatus> = {};
    for (const s of presentStatuses(snapshot, collection)) init[s] = s;
    setSelections(init);
    // Depend on collection only: opening seeds once; re-seeding on every snapshot
    // would clobber an in-progress selection.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [collection]);

  const confirm = () => {
    if (collection === null) return;
    const replacements: Record<string, string> = {};
    for (const from of rows) {
      const to = selections[from] ?? from;
      if (to !== from) replacements[from] = to;
    }
    setStatuses(replacements, collection)
      .then((snap) => { onSnapshot(snap); onClose(); })
      .catch((e) => onError(String(e)));
  };

  return (
    <Dialog.Root open={collection !== null} onOpenChange={(o) => { if (!o) onClose(); }}>
      <Dialog.Content maxWidth="440px">
        <Dialog.Title>Change Statuses{collection ? ` — ${collection}` : ""}</Dialog.Title>
        {rows.length === 0 ? (
          <Text size="2" color="gray">This collection has no items.</Text>
        ) : (
          <Flex direction="column" gap="2" mt="2">
            {rows.map((from) => (
              <Flex key={from} align="center" justify="between" gap="3">
                <Text size="2" style={{ width: 120 }}>{STATUS_LABELS[from]}</Text>
                <Text size="2" color="gray">→</Text>
                <Select.Root
                  value={selections[from] ?? from}
                  onValueChange={(v) => setSelections((s) => ({ ...s, [from]: v as TaskStatus }))}
                >
                  <Select.Trigger />
                  <Select.Content>
                    {ALL_STATUSES.map((s) => (
                      <Select.Item key={s} value={s}>{STATUS_LABELS[s]}</Select.Item>
                    ))}
                  </Select.Content>
                </Select.Root>
              </Flex>
            ))}
          </Flex>
        )}
        <Flex gap="2" mt="4" justify="end">
          <Dialog.Close>
            <Button variant="soft" color="gray">Cancel</Button>
          </Dialog.Close>
          <Button onClick={confirm} disabled={rows.length === 0}>OK</Button>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
}
```

(Parity with Swift `BulkStatusChangeSheet`: one row per status with a "No Change" default — here the default option is the same status, so an unchanged row contributes nothing to `replacements` exactly like Swift's `replacement != status` filter. Swift renders all `TaskStatus.allCases` rows; per design decision 4 we render only the *present* statuses via `presentStatuses`. The `Select` options list all statuses so any remap is reachable. If 3.3.0's `Select` requires a `placeholder` on a controlled value or a different `Trigger` shape, adjust; gate = clean tsc+build.)

- [ ] **Step 2: Host the dialog in `App` + add the open handler**

Edit `src/App.tsx`. Add the import:

```tsx
import { BulkStatusDialog } from "./components/BulkStatusDialog";
```

Add the open-state next to `promptCollection`:

```tsx
  const [bulkStatusCollection, setBulkStatusCollection] = useState<string | null>(null);
```

Mount the dialog next to `<PromptEditorDialog … />`:

```tsx
      <BulkStatusDialog
        collection={bulkStatusCollection}
        snapshot={snapshot}
        onClose={() => setBulkStatusCollection(null)}
        onSnapshot={apply}
        onError={onError}
      />
```

Pass an `onChangeStatuses` callback to `<Sidebar …>` (alongside `onEditPrompt`):

```tsx
        onChangeStatuses={(name) => setBulkStatusCollection(name)}
```

- [ ] **Step 3: Add the "Change Statuses…" item to the Sidebar collection menu**

Edit `src/components/Sidebar.tsx`. Add `onChangeStatuses` to `SidebarProps`:

```tsx
  onChangeStatuses: (name: string) => void;
```

Destructure it in the function signature (add to the existing destructure list alongside `onEditPrompt`):

```tsx
  onOpenSettings, onEditPrompt, onChangeStatuses, onSnapshot, onError, onRequestConfirm,
```

In the per-collection `ContextMenu.Content`, add a **Change Statuses…** item immediately after the `Export Collection` `Sub` and before the `Separator` that precedes `Delete`:

```tsx
                  <ContextMenu.Item onSelect={() => onChangeStatuses(c.name)}>
                    Change Statuses…
                  </ContextMenu.Item>
```

(Parity: Swift exposes bulk-status change at the `.collection` scope from the collection menu; here it opens the dialog seeded from the live snapshot.)

- [ ] **Step 4: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green. (Report any `Select`/`Dialog` 3.3.0 adjustment.)

- [ ] **Step 5: Commit**

```bash
git add src/components/BulkStatusDialog.tsx src/App.tsx src/components/Sidebar.tsx
git commit -m "feat(ui): bulk-status dialog + collection-menu Change Statuses

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 9: File-drop-to-create + final gate

Subscribe to the window drag-drop event in `App`; on a drop carrying file paths, create one Draft per file titled with the filename (target = the selected collection, or default on "All"), replacing the snapshot from the last result. Then run the full workspace + frontend gate (this is the last task of the final phase). Port of Swift `handleFileDrop` → `createTaskFromDroppedFile`.

**Files:**
- Modify: `src/App.tsx`, and (only if manual launch shows no drop event) `src-tauri/tauri.conf.json`

- [ ] **Step 1: Add a basename helper + the drag-drop subscription in `App`**

Edit `src/App.tsx`. The file already imports `getCurrentWindow` from `@tauri-apps/api/window` and `createItem` from the client.

Add a small basename helper near the top (module scope, after the imports):

```tsx
// Last path segment of a (posix or windows) absolute path — the dropped file's name.
function basename(p: string): string {
  const parts = p.split(/[/\\]/);
  return parts[parts.length - 1] || p;
}
```

Add a drag-drop subscription effect inside `App` (place it with the other `useEffect`s, e.g. after the keyboard handler effect). It must read the *current* selected collection without re-subscribing on every selection change, so it uses `viewRef` (already maintained in this file):

```tsx
  // Drop files onto the window → one Draft per file, titled with the filename.
  // Target = the selected collection ("All" → default). Mirrors Swift handleFileDrop.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    getCurrentWindow()
      .onDragDropEvent((event) => {
        if (event.payload.type !== "drop") return;
        const paths = event.payload.paths;
        if (!paths || paths.length === 0) return;
        const sel = viewRef.current.selected;
        const target = sel === ALL_COLLECTION ? undefined : sel;
        // Create sequentially so the final snapshot reflects every new Draft.
        let chain: Promise<Snapshot> = Promise.resolve(snapRef.current);
        for (const p of paths) {
          chain = chain.then(() => createItem(target, basename(p)));
        }
        chain.then(setSnapshot).catch((err) => onError(String(err)));
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch((e) => console.error(e));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);
```

(Parity with Swift: each dropped file → `createTask(title: fileURL.lastPathComponent, collection: collectionForNewDraft)`. The `onDragDropEvent` payload discriminant is `event.payload.type === "drop"` with `event.payload.paths: string[]`; verify the exact spelling against the installed `@tauri-apps/api/window` types — gate = clean tsc + the drop firing at manual launch. The drag-drop event needs no capability beyond the existing `core:default` per the Conventions block.)

- [ ] **Step 2: (Conditional) enable `dragDropEnabled` only if manual launch shows no drop**

The window drag-drop event is enabled by default in Tauri v2, so **make no config change in this step by default**. If, at manual `cargo tauri dev` launch, dropping a file produces no event (e.g. the webview swallows it), set it explicitly — edit `src-tauri/tauri.conf.json` `app.windows[0]` to add `"dragDropEnabled": true`:

```json
    "windows": [{ "label": "main", "title": "Pond", "width": 900, "height": 600, "minWidth": 480, "minHeight": 320, "dragDropEnabled": true }],
```

(Verify the key name against `https://schema.tauri.app/config/2`; this flag's effect is a manual-launch check, not a unit test. If no change is needed, skip — and note that in the commit.)

- [ ] **Step 3: Full final gate (workspace + frontend)**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p pond-tauri
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: fmt/clippy clean; all `pond-tauri` tests green (incl. `install::tests`, `mutations` set_statuses + map-deserialize + create_item title cases); `tsc`/`build` clean; all Vitest green (incl. `bulkStatus.test.ts` + the `client.test.ts` additions).

- [ ] **Step 4: Commit**

```bash
git add src/App.tsx
git commit -m "feat(ui): file-drop creates a Draft per file (titled with the filename)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

(If Step 2 changed `tauri.conf.json`, add it to the `git add` and mention it in the message body.)

---

## Done criteria

- `cargo test -p pond-tauri` green: `install::tests` (4: `target_beside`, `dto_from` round-trip, installed-cannot-install, install/uninstall flip), `mutations` (`set_statuses_remaps_within_collection`, `replacements_map_deserializes_from_json_object`, `create_item_title_none_is_empty_draft`, `create_item_title_some_creates_titled_draft`); `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- `npx vitest run` green (incl. the new `client.test.ts` cases + `bulkStatus.test.ts`); `npx tsc --noEmit` clean; `npm run build` succeeds.
- The sidecar build-copy script (`scripts/build-sidecar.mjs debug`) produces an executable `src-tauri/binaries/taskpond-<host-triple>` (gitignored); `tauri.conf.json` declares `bundle.externalBin: ["binaries/taskpond"]` and runs the build-copy in `beforeDevCommand`/`beforeBuildCommand`; `cargo build -p pond-tauri` stays clean.
- **User verification (per design decision 1):** `cargo tauri build` produces a bundle whose `taskpond` sidecar sits next to the main exe, and the Settings Command tab can install `~/.local/bin/taskpond` from the installed `.app`.
- Manual `cargo tauri dev` launch (human check): the Settings **Command** tab shows the link, status, and (when not in PATH) the PATH hint + copy; Install/Reinstall + Uninstall toggle `~/.local/bin/taskpond` (in dev, the target is `target/debug/taskpond`); the collection menu's **Change Statuses…** opens the dialog (rows = present statuses), remapping a status updates the list and is a no-op when nothing changed; dropping files onto the window creates one Draft per file titled with the filename, in the selected collection (default on "All"); a forced command error surfaces (Command tab inline; bulk-status/file-drop via the error dialog).
