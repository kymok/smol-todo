# Phase 3: Tauri Shell + IPC + Radix Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up a Tauri v2 desktop app (`pond-tauri` crate + Vite/React/TypeScript/Radix Themes frontend) that renders the `pond-core` store **read-only** — sidebar + task list, client-side selection/search/filter, and live-reload via a file-watcher.

**Architecture:** A `pond-tauri` crate under `src-tauri/` (workspace member) exposes one `get_snapshot` command over `pond-core` and runs a `notify` watcher that emits `store-changed`. The frontend (root `src/`) fetches the snapshot on mount + on that event and renders it; all view state is client-side. `pond-core` is the single source of truth — no mutations this phase.

**Tech Stack:** Rust 1.96 (pinned), Tauri v2, `serde`/`serde_json`, `notify`; Vite + React 18 + TypeScript + `@radix-ui/themes` + `@radix-ui/react-icons`; `@tauri-apps/api` v2; Vitest. npm.

**Conventions (every task):**
- Run Rust from the repo root: `cargo test -p pond-tauri`, `cargo build -p pond-tauri`. Frontend from the repo root too (Vite root is the repo root): `npm run build`, `npx tsc --noEmit`, `npx vitest run`.
- Keep `cargo clippy -p pond-tauri -- -D warnings` and `cargo fmt` clean for Rust.
- **Framework boilerplate (Tasks 2–3, 9):** the config/code below is best-effort for the pinned versions. If the toolchain reports a schema/API drift (Tauri v2 conf keys, `@tauri-apps/api` import paths, capability identifiers), apply the minimal documented fix and **report it** — the gate for these tasks is a clean build/typecheck, not byte-identical config.
- **TDD applies to logic tasks (4–8):** DTO mapping, `get_snapshot`, watcher, `view.ts`, the api client. The display-only React components (Task 9) are verified by `tsc`/`vite build` (no DOM/visual tests — per the spec).

---

## File Structure

```
rust-toolchain.toml                 channel = "1.96.0"
package.json, vite.config.ts, tsconfig.json, tsconfig.node.json, index.html
src/                                 frontend (Vite root)
├─ main.tsx                         React entry + <Theme> + styles.css
├─ App.tsx                          fetch snapshot + store-changed; view state; render
├─ api/{types.ts, client.ts}        TS DTO types; typed invoke/listen wrapper
├─ state/view.ts                    pure view-model logic (filter/group)
└─ components/{Sidebar,DetailPane,TaskRow}.tsx
src-tauri/                           pond-tauri crate (workspace member)
├─ Cargo.toml, build.rs, tauri.conf.json
├─ capabilities/default.json, icons/icon.png
└─ src/{main.rs, dto.rs, commands.rs, watcher.rs}
```

Responsibilities: `dto.rs` = wire types + mapping (pure, tested); `commands.rs` = the thin `get_snapshot`; `watcher.rs` = notify→event (tested via a channel); `main.rs` = Tauri builder wiring. Frontend: `api/` is the only `invoke` site; `state/view.ts` is pure (tested); components are thin renderers.

---

## Task 1: Pin the Rust toolchain

**Files:**
- Create: `rust-toolchain.toml`

- [ ] **Step 1: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "1.96.0"
profile = "minimal"
components = ["clippy", "rustfmt"]
```

- [ ] **Step 2: Install + verify the existing workspace on the new toolchain**

Run (from repo root; first run downloads 1.96.0 via rustup — network expected):
```bash
rustc --version   # should now report 1.96.0 inside this repo
cargo test
```
Expected: the toolchain resolves to 1.96.0 and **all existing tests pass** (pond-core 71 + taskpond-cli 19 unit + 1 integration = 91). The 1.72-era pins (`getrandom`/`tempfile`/`clap`) still build fine on 1.96 — leave them as-is (relaxing them is out of scope).

- [ ] **Step 3: Confirm lints still clean**

```bash
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check
```
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add rust-toolchain.toml
git commit -m "build: pin Rust 1.96 toolchain for Tauri v2"
```

---

## Task 2: Frontend scaffold (Vite + React + TS + Radix + Vitest)

**Files:**
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `tsconfig.node.json`, `index.html`
- Create: `src/main.tsx`, `src/App.tsx`, `src/sanity.test.ts`
- Modify: `.gitignore`

- [ ] **Step 1: `package.json`**

```json
{
  "name": "pond-frontend",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "@radix-ui/react-icons": "^1.3.0",
    "@radix-ui/themes": "^3.1.0",
    "@tauri-apps/api": "^2.0.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@types/react": "^18.3.3",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.1",
    "jsdom": "^24.1.0",
    "typescript": "^5.5.0",
    "vite": "^5.4.0",
    "vitest": "^2.0.0"
  }
}
```

- [ ] **Step 2: `vite.config.ts`** (Tauri expects a fixed dev port; `clearScreen:false` keeps Rust logs visible)

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { outDir: "dist", target: "es2021" },
  test: { environment: "jsdom" },
});
```

- [ ] **Step 3: `tsconfig.json` and `tsconfig.node.json`**

`tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2021",
    "useDefineForClassFields": true,
    "lib": ["ES2021", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noEmit": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "types": ["vitest/globals"]
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```
`tsconfig.node.json`:
```json
{
  "compilerOptions": { "composite": true, "module": "ESNext", "moduleResolution": "Bundler", "skipLibCheck": true },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 4: `index.html`** (repo root)

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Pond</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 5: `src/main.tsx` and a minimal `src/App.tsx`**

`src/main.tsx`:
```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import "@radix-ui/themes/styles.css";
import { App } from "./App";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Theme>
      <App />
    </Theme>
  </React.StrictMode>,
);
```
`src/App.tsx` (placeholder; replaced in Task 9):
```tsx
import { Heading } from "@radix-ui/themes";

export function App() {
  return <Heading>Pond</Heading>;
}
```

- [ ] **Step 6: `src/sanity.test.ts`** (proves Vitest runs)

```ts
import { describe, expect, it } from "vitest";

describe("frontend toolchain", () => {
  it("runs vitest", () => {
    expect(1 + 1).toBe(2);
  });
});
```

- [ ] **Step 7: Update `.gitignore`** — append:

```gitignore

# Node / frontend build output
node_modules/
dist/
```

- [ ] **Step 8: Install, build, typecheck, test**

```bash
npm install
npx tsc --noEmit
npx vitest run
npm run build
```
Expected: install succeeds; `tsc` clean; vitest 1 passed; `vite build` writes `dist/`. (If a dependency version above isn't resolvable, pick the nearest resolvable minor and report it.)

- [ ] **Step 9: Commit**

```bash
git add package.json package-lock.json vite.config.ts tsconfig.json tsconfig.node.json index.html src/main.tsx src/App.tsx src/sanity.test.ts .gitignore
git commit -m "feat(ui): scaffold Vite + React + Radix Themes frontend"
```

---

## Task 3: `pond-tauri` crate scaffold (Tauri v2)

**Files:**
- Modify: `Cargo.toml` (workspace members)
- Create: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/icons/icon.png`, `src-tauri/src/main.rs`

- [ ] **Step 1: Add the crate to the workspace** — set root `Cargo.toml` `[workspace] members`:

```toml
members = ["crates/pond-core", "crates/taskpond-cli", "src-tauri"]
```
(Keep the existing `resolver` and `[workspace.dependencies]` block unchanged.)

- [ ] **Step 2: `src-tauri/Cargo.toml`**

```toml
[package]
name = "pond-tauri"
version = "0.1.0"
edition = "2021"
description = "Pond desktop app"

[lib]
name = "pond_tauri_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
pond-core = { path = "../crates/pond-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
notify = "6"
```

- [ ] **Step 3: `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build();
}
```

- [ ] **Step 4: `src-tauri/tauri.conf.json`** (v2 schema; the window label `main` must match the capability)

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Pond",
  "version": "0.1.0",
  "identifier": "dev.kymok.pond",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [{ "label": "main", "title": "Pond", "width": 900, "height": 600, "minWidth": 480, "minHeight": 320 }],
    "security": { "csp": null }
  },
  "bundle": { "active": true, "targets": "all", "icon": ["icons/icon.png"] }
}
```

- [ ] **Step 5: `src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capability for the main window",
  "windows": ["main"],
  "permissions": ["core:default"]
}
```

- [ ] **Step 6: Add an icon** — create a small valid PNG at `src-tauri/icons/icon.png` (any 32×32+ RGBA PNG). If none is handy:
```bash
mkdir -p src-tauri/icons
# Minimal 1x1 PNG (sufficient for a debug build; replace before bundling):
printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82' > src-tauri/icons/icon.png
```
(If `tauri-build`/bundling rejects the 1×1 icon, drop in a normal app icon and report it. It does not affect `cargo build`.)

- [ ] **Step 7: `src-tauri/src/main.rs`** (minimal builder; command + watcher wired in Tasks 5–6)

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running pond-tauri");
}
```

- [ ] **Step 8: Build the crate** (frontend `dist/` must exist — Task 2 built it; rebuild if needed)

```bash
npm run build
cargo build -p pond-tauri
```
Expected: `tauri-build` runs and `cargo build -p pond-tauri` succeeds. (`cargo tauri dev` is for the manual launch in Task 10 and needs the Tauri CLI; it is not required to build the crate here. If `generate_context!` complains that `dist/` is missing, run `npm run build` first.)

- [ ] **Step 9: Lint + commit**

```bash
cargo clippy -p pond-tauri -- -D warnings && cargo fmt
git add Cargo.toml Cargo.lock src-tauri
git commit -m "feat(app): scaffold pond-tauri (Tauri v2) crate"
```

---

## Task 4: Snapshot DTOs + mapping (`dto.rs`)

**Files:**
- Create: `src-tauri/src/dto.rs`
- Modify: `src-tauri/src/main.rs` (declare `mod dto;`)

The DTOs hold `pond-core` enums directly (`CollectionColor`/`TaskStatus` already serialize to their rawValues) and rename to camelCase. `TaskItem` is sent as-is (it already serializes camelCase).

- [ ] **Step 1: Write the failing test** — create `src-tauri/src/dto.rs`

```rust
use pond_core::{CollectionColor, CollectionGroupSummary, CollectionSummary, TaskItem, TaskStatus};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionSummaryDto {
    pub name: String,
    pub display_name: String,
    pub group_name: String,
    pub total_count: usize,
    pub incomplete_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_indicator: Option<TaskStatus>,
    pub color: CollectionColor,
    pub is_archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_template: Option<String>,
}

impl From<&CollectionSummary> for CollectionSummaryDto {
    fn from(s: &CollectionSummary) -> Self {
        CollectionSummaryDto {
            name: s.name.clone(),
            display_name: s.display_name.clone(),
            group_name: s.group_name.clone(),
            total_count: s.total_count,
            incomplete_count: s.incomplete_count,
            status_indicator: s.status_indicator,
            color: s.color,
            is_archived: s.is_archived,
            prompt_template: s.prompt_template.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionGroupSummaryDto {
    pub name: String,
    pub collections: Vec<CollectionSummaryDto>,
}

impl From<&CollectionGroupSummary> for CollectionGroupSummaryDto {
    fn from(g: &CollectionGroupSummary) -> Self {
        CollectionGroupSummaryDto {
            name: g.name.clone(),
            collections: g.collections.iter().map(CollectionSummaryDto::from).collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDto {
    pub items: Vec<TaskItem>,
    pub collections: Vec<CollectionSummaryDto>,
    pub groups: Vec<CollectionGroupSummaryDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_dto_is_camel_case_with_raw_values() {
        let summary = CollectionSummary {
            name: "Work/A".into(),
            display_name: "A".into(),
            group_name: "Work".into(),
            total_count: 3,
            incomplete_count: 2,
            status_indicator: Some(TaskStatus::OnHold),
            color: CollectionColor::Blue,
            is_archived: false,
            prompt_template: None,
        };
        let json = serde_json::to_string(&CollectionSummaryDto::from(&summary)).unwrap();
        assert!(json.contains("\"displayName\":\"A\""));
        assert!(json.contains("\"groupName\":\"Work\""));
        assert!(json.contains("\"incompleteCount\":2"));
        assert!(json.contains("\"color\":\"blue\""));
        assert!(json.contains("\"statusIndicator\":\"on-hold\""));
        assert!(!json.contains("promptTemplate")); // None omitted
    }
}
```

- [ ] **Step 2: Declare the module** — add to the top of `src-tauri/src/main.rs` (above `fn main`):

```rust
mod dto;
```
Since `main.rs` doesn't use `dto` yet, add `#[allow(dead_code)]` on the `mod dto;` line **only if** clippy flags unused — it will be used in Task 5; report if added (removed implicitly when Task 5 references it).

- [ ] **Step 3: Run the test**

Run: `cargo test -p pond-tauri dto`
Expected: PASS (`collection_dto_is_camel_case_with_raw_values`).

- [ ] **Step 4: Lint + commit**

```bash
cargo clippy -p pond-tauri --all-targets -- -D warnings && cargo fmt
git add src-tauri/src/dto.rs src-tauri/src/main.rs
git commit -m "feat(app): add snapshot DTOs"
```

---

## Task 5: `get_snapshot` command (`commands.rs`)

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`

Split into a testable inner `build_snapshot(&TaskStore)` and the thin `#[tauri::command]` wrapper.

- [ ] **Step 1: Write the failing test** — create `src-tauri/src/commands.rs`

```rust
use crate::dto::{CollectionGroupSummaryDto, CollectionSummaryDto, SnapshotDto};
use pond_core::{Result, TaskStore};

/// Build the full read-only snapshot from a store. Testable (no Tauri types).
pub fn build_snapshot(store: &TaskStore) -> Result<SnapshotDto> {
    Ok(SnapshotDto {
        items: store.items(None, None, &[], None)?,
        collections: store
            .collection_summaries()?
            .iter()
            .map(CollectionSummaryDto::from)
            .collect(),
        groups: store
            .collection_group_summaries()?
            .iter()
            .map(CollectionGroupSummaryDto::from)
            .collect(),
    })
}

#[tauri::command]
pub fn get_snapshot() -> std::result::Result<SnapshotDto, String> {
    build_snapshot(&TaskStore::open_default()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pond_core::TaskStatus;
    use tempfile::tempdir;

    #[test]
    fn build_snapshot_reflects_store_contents() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("Ship it", "Work/Docs", None, false, TaskStatus::Ready).unwrap();

        let snap = build_snapshot(&store).unwrap();
        assert_eq!(snap.items.len(), 1);
        assert_eq!(snap.items[0].title, "Ship it");
        assert!(snap.collections.iter().any(|c| c.name == "Work/Docs"));
        assert!(snap.groups.iter().any(|g| g.name == "Work"));
    }
}
```

- [ ] **Step 2: Add `tempfile` as a dev-dependency** — under a new `[dev-dependencies]` in `src-tauri/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 3: Declare the module + register the command** — set `src-tauri/src/main.rs` to:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod dto;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::get_snapshot])
        .run(tauri::generate_context!())
        .expect("error while running pond-tauri");
}
```

- [ ] **Step 4: Run the test + build**

```bash
cargo test -p pond-tauri commands
npm run build && cargo build -p pond-tauri
```
Expected: test PASS; crate builds with the command registered.

- [ ] **Step 5: Lint + commit**

```bash
cargo clippy -p pond-tauri --all-targets -- -D warnings && cargo fmt
git add src-tauri/Cargo.toml Cargo.lock src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(app): add get_snapshot command"
```

---

## Task 6: Store-change watcher (`watcher.rs`) + wire into `main.rs`

**Files:**
- Create: `src-tauri/src/watcher.rs`
- Modify: `src-tauri/src/main.rs`

`watch_dir` is testable (calls a plain callback); `main.rs` passes a callback that emits `store-changed`.

- [ ] **Step 1: Write the failing test** — create `src-tauri/src/watcher.rs`

```rust
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

/// Watch `dir` and invoke `on_change` (debounced by `settle`) on any filesystem event.
/// Returns the watcher (kept alive by the caller) — dropping it stops watching.
pub fn watch_dir<F>(dir: &Path, settle: Duration, on_change: F) -> notify::Result<RecommendedWatcher>
where
    F: Fn() + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if res.is_ok() {
            let _ = tx.send(());
        }
    })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;

    // Debounce: coalesce a burst of events into one callback after `settle`.
    std::thread::spawn(move || loop {
        match rx.recv() {
            Ok(()) => {
                while rx.recv_timeout(settle).is_ok() {}
                on_change();
            }
            Err(_) => break,
        }
    });
    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn fires_callback_on_file_change() {
        let dir = tempdir().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        let _watcher = watch_dir(dir.path(), Duration::from_millis(50), move || {
            c.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();

        std::fs::write(dir.path().join("tasks.json"), b"{}").unwrap();
        // Allow the event to propagate + debounce window to elapse.
        std::thread::sleep(Duration::from_millis(600));
        assert!(count.load(Ordering::SeqCst) >= 1, "watcher should fire at least once");
    }
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p pond-tauri watcher`
Expected: PASS. (If flaky on a slow filesystem, the 600 ms sleep can be raised — but do not lower the assertion below `>= 1`.)

- [ ] **Step 3: Wire the watcher + event into `main.rs`** — set `src-tauri/src/main.rs` to:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod dto;
mod watcher;

use std::time::Duration;
use tauri::{Emitter, Manager};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::get_snapshot])
        .setup(|app| {
            let store_dir = pond_core::paths::default_store_path()
                .parent()
                .map(|p| p.to_path_buf());
            if let Some(dir) = store_dir {
                std::fs::create_dir_all(&dir).ok();
                let handle = app.handle().clone();
                // Keep the watcher alive for the app's lifetime.
                match watcher::watch_dir(&dir, Duration::from_millis(150), move || {
                    let _ = handle.emit("store-changed", ());
                }) {
                    Ok(w) => {
                        app.manage(w);
                    }
                    Err(e) => eprintln!("store watcher failed to start: {e}"),
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running pond-tauri");
}
```

- [ ] **Step 4: Build**

```bash
npm run build && cargo build -p pond-tauri
```
Expected: builds. (If `app.manage` requires `RecommendedWatcher: Send + Sync` and the chosen `notify` backend isn't `Sync`, wrap it: `app.manage(std::sync::Mutex::new(w));` — report if needed.)

- [ ] **Step 5: Lint + commit**

```bash
cargo clippy -p pond-tauri --all-targets -- -D warnings && cargo fmt
git add src-tauri/src/watcher.rs src-tauri/src/main.rs
git commit -m "feat(app): add store-changed file watcher"
```

---

## Task 7: Frontend API client (`api/types.ts`, `api/client.ts`)

**Files:**
- Create: `src/api/types.ts`, `src/api/client.ts`, `src/api/client.test.ts`

- [ ] **Step 1: `src/api/types.ts`** (mirror the DTOs)

```ts
export type TaskStatus =
  | "draft" | "ready" | "in-progress" | "completed" | "on-hold" | "rejected" | "aborted";
export type CollectionColor =
  | "gray" | "red" | "orange" | "yellow" | "green" | "blue" | "purple";

export interface TaskNote { id: string; version: string; body: string; createdAt: string; updatedAt: string; }
export interface TaskItem {
  id: string; version: string; title: string; collection: string;
  note?: TaskNote; status: TaskStatus; createdAt: string; updatedAt: string;
}
export interface CollectionSummary {
  name: string; displayName: string; groupName: string;
  totalCount: number; incompleteCount: number;
  statusIndicator?: TaskStatus; color: CollectionColor; isArchived: boolean; promptTemplate?: string;
}
export interface CollectionGroupSummary { name: string; collections: CollectionSummary[]; }
export interface Snapshot { items: TaskItem[]; collections: CollectionSummary[]; groups: CollectionGroupSummary[]; }
```

- [ ] **Step 2: Write the failing test** — create `src/api/client.test.ts`

```ts
import { describe, expect, it, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();
const listenMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invokeMock(...a) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: (...a: unknown[]) => listenMock(...a) }));

import { getSnapshot, onStoreChanged } from "./client";

describe("api client", () => {
  beforeEach(() => { invokeMock.mockReset(); listenMock.mockReset(); });

  it("getSnapshot invokes the get_snapshot command", async () => {
    invokeMock.mockResolvedValue({ items: [], collections: [], groups: [] });
    const snap = await getSnapshot();
    expect(invokeMock).toHaveBeenCalledWith("get_snapshot");
    expect(snap.items).toEqual([]);
  });

  it("onStoreChanged registers a store-changed listener", async () => {
    const unlisten = vi.fn();
    listenMock.mockResolvedValue(unlisten);
    const cb = vi.fn();
    await onStoreChanged(cb);
    expect(listenMock).toHaveBeenCalledWith("store-changed", expect.any(Function));
  });
});
```

- [ ] **Step 3: Run it to confirm it fails**

Run: `npx vitest run src/api/client.test.ts`
Expected: FAIL ("Cannot find module './client'").

- [ ] **Step 4: `src/api/client.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Snapshot } from "./types";

export function getSnapshot(): Promise<Snapshot> {
  return invoke<Snapshot>("get_snapshot");
}

export function onStoreChanged(callback: () => void): Promise<UnlistenFn> {
  return listen("store-changed", () => callback());
}
```

- [ ] **Step 5: Run tests + typecheck**

```bash
npx vitest run src/api/client.test.ts
npx tsc --noEmit
```
Expected: 2 passed; tsc clean.

- [ ] **Step 6: Commit**

```bash
git add src/api
git commit -m "feat(ui): add typed snapshot/event API client"
```

---

## Task 8: Frontend view logic (`state/view.ts`)

**Files:**
- Create: `src/state/view.ts`, `src/state/view.test.ts`

Pure functions — the read-only equivalents of `pond-core`'s visibility + sidebar logic, computed client-side.

- [ ] **Step 1: Write the failing test** — create `src/state/view.test.ts`

```ts
import { describe, expect, it } from "vitest";
import type { Snapshot } from "../api/types";
import { ALL_COLLECTION, visibleItems, allIncompleteCount } from "./view";

function snap(): Snapshot {
  const base = { version: "v", createdAt: "t", updatedAt: "t" } as const;
  return {
    items: [
      { ...base, id: "00000001", title: "alpha", collection: "Inbox", status: "ready" },
      { ...base, id: "00000002", title: "beta", collection: "Work/A", status: "completed" },
      { ...base, id: "00000003", title: "gamma", collection: "Inbox", status: "in-progress" },
    ],
    collections: [], groups: [],
  };
}

describe("visibleItems", () => {
  it("ALL shows everything; a collection filters", () => {
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "", incompleteOnly: false }).length).toBe(3);
    expect(visibleItems(snap(), { selected: "Inbox", search: "", incompleteOnly: false }).map(i => i.id))
      .toEqual(["00000001", "00000003"]);
  });

  it("search matches title/collection/id and is case-insensitive", () => {
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "BETA", incompleteOnly: false }).map(i => i.id))
      .toEqual(["00000002"]);
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "work", incompleteOnly: false }).map(i => i.id))
      .toEqual(["00000002"]);
  });

  it("incompleteOnly hides completed", () => {
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "", incompleteOnly: true }).map(i => i.id))
      .toEqual(["00000001", "00000003"]);
  });

  it("allIncompleteCount counts non-completed", () => {
    expect(allIncompleteCount(snap())).toBe(2);
  });
});
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `npx vitest run src/state/view.test.ts`
Expected: FAIL (module not found).

- [ ] **Step 3: `src/state/view.ts`**

```ts
import type { CollectionGroupSummary, Snapshot, TaskItem } from "../api/types";

export const ALL_COLLECTION = "__all__";

export interface ViewState {
  selected: string; // ALL_COLLECTION or a collection name
  search: string;
  incompleteOnly: boolean;
}

function matchesSearch(item: TaskItem, query: string): boolean {
  if (!query) return true;
  const q = query.toLowerCase();
  return (
    item.title.toLowerCase().includes(q) ||
    item.collection.toLowerCase().includes(q) ||
    item.id.toLowerCase().includes(q) ||
    (item.note?.body.toLowerCase().includes(q) ?? false)
  );
}

export function visibleItems(snapshot: Snapshot, view: ViewState): TaskItem[] {
  return snapshot.items.filter((item) => {
    const collectionMatches = view.selected === ALL_COLLECTION || item.collection === view.selected;
    const statusMatches = !view.incompleteOnly || item.status !== "completed";
    return collectionMatches && statusMatches && matchesSearch(item, view.search);
  });
}

export function allIncompleteCount(snapshot: Snapshot): number {
  return snapshot.items.filter((i) => i.status !== "completed").length;
}

/** Sidebar groups, optionally hiding archived collections (default: hide). */
export function sidebarGroups(snapshot: Snapshot, showArchived: boolean): CollectionGroupSummary[] {
  return snapshot.groups
    .map((g) => ({ ...g, collections: g.collections.filter((c) => showArchived || !c.isArchived) }))
    .filter((g) => g.collections.length > 0);
}
```

- [ ] **Step 4: Run tests + typecheck**

```bash
npx vitest run src/state/view.test.ts
npx tsc --noEmit
```
Expected: 4 passed; tsc clean.

- [ ] **Step 5: Commit**

```bash
git add src/state
git commit -m "feat(ui): add read-only view-model logic"
```

---

## Task 9: Components + App wiring (display-only)

**Files:**
- Create: `src/components/Sidebar.tsx`, `src/components/DetailPane.tsx`, `src/components/TaskRow.tsx`
- Modify: `src/App.tsx`

No DOM/visual tests (per the spec); the gate is `tsc --noEmit` + `vite build`. Status/collection colors use the Radix `color` prop (built-in palette).

- [ ] **Step 1: `src/components/TaskRow.tsx`**

```tsx
import { Badge, Flex, Text } from "@radix-ui/themes";
import { DotFilledIcon } from "@radix-ui/react-icons";
import type { CollectionColor, TaskItem, TaskStatus } from "../api/types";

const STATUS_COLOR: Record<TaskStatus, CollectionColor> = {
  draft: "gray", ready: "gray", "in-progress": "blue", completed: "green",
  "on-hold": "orange", rejected: "red", aborted: "red",
};

export function TaskRow({ item, showCollection }: { item: TaskItem; showCollection: boolean }) {
  const dim = item.status === "completed" || item.status === "in-progress";
  return (
    <Flex align="start" gap="2" py="1">
      <Text color={STATUS_COLOR[item.status]}><DotFilledIcon /></Text>
      <Flex direction="column" flexGrow="1">
        <Text size="2" color={dim ? "gray" : undefined}>{item.title || "Untitled"}</Text>
        {item.note ? <Text size="1" color="gray">{item.note.body}</Text> : null}
      </Flex>
      {showCollection ? <Badge color="gray" variant="soft">{item.collection}</Badge> : null}
    </Flex>
  );
}
```

- [ ] **Step 2: `src/components/Sidebar.tsx`**

```tsx
import { Badge, Box, Button, Flex, Text } from "@radix-ui/themes";
import { DotFilledIcon } from "@radix-ui/react-icons";
import type { Snapshot } from "../api/types";
import { ALL_COLLECTION, allIncompleteCount, sidebarGroups } from "../state/view";

export function Sidebar({
  snapshot, selected, onSelect,
}: { snapshot: Snapshot; selected: string; onSelect: (name: string) => void }) {
  return (
    <Flex direction="column" gap="1" p="2" style={{ width: 240 }}>
      <Button variant={selected === ALL_COLLECTION ? "soft" : "ghost"} onClick={() => onSelect(ALL_COLLECTION)}>
        <Flex align="center" gap="2" flexGrow="1">
          <Text flexGrow="1" align="left">All</Text>
          <Badge>{allIncompleteCount(snapshot)}</Badge>
        </Flex>
      </Button>
      {sidebarGroups(snapshot, false).map((group) => (
        <Box key={group.name} mt="2">
          <Text size="1" color="gray">{group.name === "DefaultGroup" ? "No Group" : group.name}</Text>
          {group.collections.map((c) => (
            <Button key={c.name} variant={selected === c.name ? "soft" : "ghost"} onClick={() => onSelect(c.name)}>
              <Flex align="center" gap="2" flexGrow="1">
                <Text color={c.color}><DotFilledIcon /></Text>
                <Text flexGrow="1" align="left">{c.displayName}</Text>
                <Badge>{c.incompleteCount}</Badge>
              </Flex>
            </Button>
          ))}
        </Box>
      ))}
    </Flex>
  );
}
```

- [ ] **Step 3: `src/components/DetailPane.tsx`**

```tsx
import { Flex, Heading, ScrollArea, TextField } from "@radix-ui/themes";
import { MagnifyingGlassIcon } from "@radix-ui/react-icons";
import type { Snapshot } from "../api/types";
import { ALL_COLLECTION, visibleItems, type ViewState } from "../state/view";
import { TaskRow } from "./TaskRow";

export function DetailPane({
  snapshot, view, onSearch,
}: { snapshot: Snapshot; view: ViewState; onSearch: (q: string) => void }) {
  const items = visibleItems(snapshot, view);
  const title = view.selected === ALL_COLLECTION
    ? "All"
    : snapshot.collections.find((c) => c.name === view.selected)?.displayName ?? view.selected;
  return (
    <Flex direction="column" flexGrow="1" p="3" gap="3">
      <Flex align="center" justify="between">
        <Heading size="4">{title}</Heading>
        <TextField.Root placeholder="Search" value={view.search} onChange={(e) => onSearch(e.target.value)}>
          <TextField.Slot><MagnifyingGlassIcon /></TextField.Slot>
        </TextField.Root>
      </Flex>
      <ScrollArea>
        <Flex direction="column">
          {items.map((item) => (
            <TaskRow key={item.id} item={item} showCollection={view.selected === ALL_COLLECTION} />
          ))}
        </Flex>
      </ScrollArea>
    </Flex>
  );
}
```

- [ ] **Step 4: `src/App.tsx`** (replace the placeholder)

```tsx
import { useEffect, useState } from "react";
import { Flex } from "@radix-ui/themes";
import type { Snapshot } from "./api/types";
import { getSnapshot, onStoreChanged } from "./api/client";
import { ALL_COLLECTION, type ViewState } from "./state/view";
import { Sidebar } from "./components/Sidebar";
import { DetailPane } from "./components/DetailPane";

const EMPTY: Snapshot = { items: [], collections: [], groups: [] };

export function App() {
  const [snapshot, setSnapshot] = useState<Snapshot>(EMPTY);
  const [view, setView] = useState<ViewState>({ selected: ALL_COLLECTION, search: "", incompleteOnly: false });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const refresh = () => { getSnapshot().then(setSnapshot).catch((e) => console.error(e)); };
    refresh();
    onStoreChanged(refresh).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, []);

  return (
    <Flex height="100vh">
      <Sidebar snapshot={snapshot} selected={view.selected} onSelect={(name) => setView((v) => ({ ...v, selected: name }))} />
      <DetailPane snapshot={snapshot} view={view} onSearch={(q) => setView((v) => ({ ...v, search: q }))} />
    </Flex>
  );
}
```

- [ ] **Step 5: Typecheck + build (the gate)**

```bash
npx tsc --noEmit
npm run build
```
Expected: tsc clean; `vite build` succeeds. (Adjust any Radix prop that the installed `@radix-ui/themes` version names differently — e.g. `TextField.Root`/`TextField.Slot` vs `TextField`; the gate is a clean typecheck/build. Report any such adjustment.)

- [ ] **Step 6: Commit**

```bash
git add src/components src/App.tsx
git commit -m "feat(ui): read-only sidebar + detail + task rows"
```

---

## Task 10: Final gate + run docs

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Full workspace + frontend gates**

```bash
cargo test && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check
npx vitest run && npx tsc --noEmit && npm run build
```
Expected: all green — Rust (pond-core 71 + taskpond-cli 20 + pond-tauri dto/commands/watcher tests) and frontend (sanity + client + view tests), clippy/fmt clean, frontend typecheck + build clean.

- [ ] **Step 2: README — add a "Run the app" note** (after the existing Rust-workspace section)

```markdown
### Run the desktop app (Tauri)

Install the Tauri CLI once: `cargo install tauri-cli --version "^2"` (or use `npx @tauri-apps/cli`).

- Dev (hot-reload): `cargo tauri dev` — launches the window against the Vite dev server.
- The window renders the store read-only; edits made by `taskpond` (the CLI) appear live.
- The store path honors `POND_STORE`.

Frontend-only checks: `npm test` (Vitest), `npm run build` (typecheck + bundle).
```

- [ ] **Step 3: Manual launch verification (human)**

Run `cargo tauri dev`. Confirm: the window opens; the sidebar lists collections/groups with counts and color dots; selecting a collection filters the list; search filters; running `taskpond item create -c Inbox "live update test"` in another terminal makes the new task appear within ~1s (file-watcher → re-fetch). This manual check stands in for automated visual tests.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: how to run the Tauri desktop app"
```

---

## Self-Review (completed during planning)

**Spec coverage (Phase 3 design doc):**
- Toolchain `rust-toolchain.toml` 1.96.0 + re-verify → Task 1. ✅
- Manual workspace integration (`pond-tauri` crate + Vite/React/TS frontend, npm) → Tasks 2–3. ✅
- IPC `get_snapshot` + `notify` `store-changed` watcher → Tasks 5–6. ✅
- `pond-tauri`-owned camelCase DTOs (items as `TaskItem`, summary DTOs) → Task 4. ✅
- Read-only Radix shell (sidebar/detail/task-list, selection/search/incomplete-only) → Tasks 8–9. ✅
- State: re-fetch on mount + `store-changed`; view state in React → Task 9. ✅
- Verification: Rust unit tests (DTO + watcher + build_snapshot), Vitest logic tests (client + view), build gates, manual launch; **no visual-regression tests** → Tasks 4–10. ✅
- Non-goals respected: no mutations/menus/settings/always-on-top/file-drop/drag-reorder. ✅

**Placeholder scan:** No TBD/"handle errors". Framework-boilerplate tasks (2,3,6,9) include explicit, bounded "adapt version drift; gate is a clean build" instructions with the *mechanism* shown — these are deliberate, not vague placeholders. The one conditional `#[allow(dead_code)]` (Task 4) is introduced with its removal condition stated.

**Type consistency:** `SnapshotDto{items,collections,groups}` ↔ TS `Snapshot`; `CollectionSummaryDto` camelCase fields ↔ TS `CollectionSummary`; `build_snapshot(&TaskStore) -> Result<SnapshotDto>` ↔ `get_snapshot() -> Result<SnapshotDto, String>`; `getSnapshot`/`onStoreChanged`, `visibleItems`/`allIncompleteCount`/`sidebarGroups`/`ALL_COLLECTION`/`ViewState` are used consistently across `client.ts`, `view.ts`, and the components. `watch_dir(&Path, Duration, F)` matches its `main.rs` call site. `pond-core` calls (`items`, `collection_summaries`, `collection_group_summaries`, `open_default`, `paths::default_store_path`) match the Phase 1 public API.
