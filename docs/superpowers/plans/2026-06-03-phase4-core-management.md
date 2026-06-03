# Phase 4: Core Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the Phase 3 read-only Tauri shell into a day-to-day-usable editor. Add the mutation IPC layer (thin wrappers over `pond-core`) and the frontend that drives it: status changes, create/delete, notes, the full inline title/note editor (incl. merge/split), and collection/group management. Every mutation returns a fresh `SnapshotDto`; the frontend replaces its state from the result. `pond-core` already exposes every operation — **no Rust core changes**.

**Architecture:** `pond-core` stays the single source of truth. A single `TaskStore` is held in Tauri state (`app.manage`); `get_snapshot` and all mutation commands use it. A new **testable seam** `src-tauri/src/mutations.rs` holds pure `&TaskStore`-taking functions (do the `pond-core` op, then `build_snapshot(store)` → `pond_core::Result<SnapshotDto>`); `commands.rs` holds the `#[tauri::command]` wrappers (take `tauri::State<TaskStore>` + args, call the `mutations::` fn, `.map_err(|e| e.to_string())`, return `Result<SnapshotDto, String>`). The frontend has one `invoke` site (`api/client.ts`), pure tested logic in `state/` (selectors + the editor reducer), and thin components that render and dispatch intents. The `store-changed` watcher remains **only** for external (CLI) edits.

**Tech Stack:** Rust 1.96.0 (pinned), Tauri v2, `serde`/`serde_json`, `notify`, `tempfile` (dev); Vite + React 18 + TypeScript + `@radix-ui/themes` 3.3.0 + `@radix-ui/react-icons`; `@tauri-apps/api` v2; Vitest. npm.

---

## Conventions (read this section before `## File Structure`)

Every task obeys these. They are not repeated per step.

- **Branch:** work on the existing `tauri-radix-migration` branch. Do **not** create a new branch and do **not** set an upstream.
- **Rust toolchain:** pinned `1.96.0` (already in `rust-toolchain.toml`). Run all `cargo`/`npm`/`npx` commands from the repo root (the Vite root is the repo root).
- **Per Rust task gate:** `cargo fmt --all` then `cargo clippy --workspace --all-targets -- -D warnings` must be clean, and `cargo test -p pond-tauri` green.
- **Per frontend task gate:** `npx tsc --noEmit` clean, `npm run build` succeeds, `npx vitest run` green.
- **Imports/`use` at the top:** ALL `import` (TS) and `use` (Rust) statements live at the top of the file. In Rust test modules, all `use` go at the top of `mod tests` (i.e. directly under `#[cfg(test)] mod tests {`). No mid-file imports.
- **Radix Themes defaults only:** stock Radix parts, built-in named palette for status/collection colors, no theme customization. The TSX below targets the installed `@radix-ui/themes` **3.3.0** API. Where a sample's component/prop shape differs from what 3.3.0 actually exports, **adjust the usage to the installed API** — the gate is a clean `tsc --noEmit` + `npm run build` (report any adjustment in the task's commit/notes). This is the established Phase 3 latitude, **not** a placeholder to leave logic unwritten.
- **No visual / DOM / screenshot tests.** Verification is logic unit tests (Vitest reducer/selector tests, Rust command tests) + the build/typecheck gates + manual `cargo tauri dev` launch (human visual check). Do **not** add `@testing-library`/`jsdom` render tests.
- **Command (invoke) names** must equal exactly what the frontend `client.ts` wrapper passes to `invoke`.
- **Commit trailer:** every commit message ends with a trailing line:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`

### Verified `pond-core` signatures (source of truth — do not guess)

```rust
// crates/pond-core/src/store.rs (all on `impl TaskStore`, all return pond_core::Result<_>)
pub fn new<P: Into<PathBuf>>(file_path: P) -> Self
pub fn open_default() -> Self
pub fn add(&self, title: &str, collection: &str, requested_id: Option<&str>, allow_empty_title: bool, status: TaskStatus) -> Result<TaskItem>
pub fn update(&self, id: &str, title: Option<&str>, collection: Option<&str>, status: Option<TaskStatus>) -> Result<TaskItem>
pub fn update_if_current(&self, id: &str, title: Option<&str>, collection: Option<&str>, status: Option<TaskStatus>, expected: &TaskItem) -> Result<Option<TaskItem>>
pub fn move_item(&self, id: &str, collection: &str) -> Result<TaskItem>
pub fn delete(&self, id: &str) -> Result<()>
pub fn delete_if_current(&self, id: &str, expected: &TaskItem) -> Result<bool>
pub fn delete_many(&self, ids: &[String], collection: Option<&str>) -> Result<Vec<TaskItem>>
pub fn clear_items(&self, collection: &str, completed_only: bool) -> Result<Vec<TaskItem>>
pub fn set_status(&self, status: TaskStatus, ids: &[String], collection: Option<&str>) -> Result<Vec<TaskItem>>
pub fn set_status_if_current(&self, status: TaskStatus, id: &str, expected: &TaskItem) -> Result<Option<TaskItem>>
pub fn add_note(&self, id: &str, body: &str) -> Result<TaskItem>
pub fn add_note_if_current(&self, id: &str, body: &str, expected: &TaskItem) -> Result<Option<TaskItem>>
pub fn update_note(&self, id: &str, body: &str) -> Result<TaskItem>          // NO _if_current variant exists
pub fn delete_note(&self, id: &str) -> Result<TaskItem>
pub fn delete_note_if_current(&self, id: &str, expected: &TaskItem) -> Result<Option<TaskItem>>
pub fn merge_item(&self, id: &str, into_previous: &str, title: &str) -> Result<Option<TaskItem>>   // Ok(None) if prev not Draft|Ready or prev has a note, or source==prev
pub fn split_item(&self, id: &str, first_title: &str, second_title: &str, requested_second_id: Option<&str>) -> Result<TaskItem>  // InvalidTitle if either trimmed title empty
pub fn create_collection(&self, name: &str, group: &str) -> Result<String>
pub fn rename_collection(&self, old: &str, new: &str) -> Result<String>
pub fn move_collection(&self, name: &str, to_group: &str) -> Result<CollectionSummary>
pub fn set_collection_color(&self, name: &str, color: CollectionColor) -> Result<CollectionSummary>
pub fn set_collection_archived(&self, name: &str, is_archived: bool) -> Result<CollectionSummary>
pub fn delete_collection(&self, name: &str) -> Result<bool>
pub fn create_group(&self, name: &str) -> Result<String>
pub fn rename_group(&self, old: &str, new: &str) -> Result<String>
pub fn delete_group(&self, name: &str) -> Result<bool>
```

`TaskStatus` and `CollectionColor` are re-exported from `pond_core` and `#[derive(Serialize, Deserialize)]` with `rename_all="kebab-case"` (`in-progress`, `on-hold`) and `rename_all="lowercase"` respectively. **Therefore Tauri command params may be typed `TaskStatus` / `CollectionColor` directly** — serde deserializes the wire strings (the TS unions already mirror them). No manual string-parsing needed. `DEFAULT_COLLECTION` and `DEFAULT_GROUP` are re-exported from `pond_core`.

### Divergences from the design spec's IPC table (confirmed against source)

1. **`update_note` has no `_if_current` variant.** The spec table writes `update_note(id, body, ifCurrent?)`, but `pond-core` only has `update_note(&self, id, &str)`. The `update_note` command therefore **omits `ifCurrent`** and uses the plain variant. (`add_note`/`delete_note` *do* have `_if_current` variants and keep the optional param.) Concurrency for note-body edits is still protected by the editor's local-draft model.
2. **No note id on the wire.** Swift's `updateNote`/`deleteNote` take a `noteID`, but `pond-core`'s note ops address the item's single note by item id only. Commands take **no** `noteId` param.
3. **`set_status` is batch in core.** `pond-core::set_status(status, ids: &[String], collection)` takes a slice; the single-id `set_status` command builds a one-element slice (`&[id]`, collection `None`). The `_if_current` path uses `set_status_if_current(status, id, expected)`.

---

## File Structure

```
src-tauri/src/
├─ main.rs        register all commands in generate_handler!; app.manage(TaskStore::open_default()) in setup
├─ mutations.rs   (NEW) pure &TaskStore fns: do the pond-core op + build_snapshot → Result<SnapshotDto>; #[cfg(test)] tempdir tests
├─ commands.rs    get_snapshot (migrated to State<TaskStore>) + #[tauri::command] wrappers calling mutations::, mapping Err→String
├─ dto.rs         (unchanged) SnapshotDto / CollectionSummaryDto / CollectionGroupSummaryDto
└─ watcher.rs     (unchanged) store-changed for external edits

src/
├─ api/
│  ├─ client.ts   one typed wrapper per command (invoke<Snapshot>(name, args)) + getSnapshot/onStoreChanged
│  ├─ types.ts    DTO mirrors + mutation arg helper types (unchanged DTOs)
│  └─ client.test.ts  mocked-invoke assertions for a couple of wrappers
├─ state/
│  ├─ view.ts     selectors: + hideCompleted filtering, + showArchived; ViewState extended
│  ├─ view.test.ts
│  ├─ status.ts   (NEW) pure leadingStatusClickTarget + rightClickStatusTarget
│  ├─ status.test.ts (NEW)
│  ├─ editor.ts   (NEW) pure key-handling reducer (key+caret+field+composing) → EditorIntent union
│  └─ editor.test.ts (NEW)
├─ components/
│  ├─ Sidebar.tsx       + collection/group context menus, footer toggles, create/rename/delete/color/archive/move/clear
│  ├─ DetailPane.tsx    + "new task" affordance (→createItem), passes mutation callbacks down
│  ├─ TaskRow.tsx       + status leading-click, status/move/delete menu, inline title+note editor host
│  └─ InlineEditor.tsx  (NEW) Text⇄TextArea swap, local draft, 500ms IME-safe autosave, keydown→editor.ts→intent→client
└─ App.tsx        snapshot replace-from-mutation-result; Cmd+N / Cmd+Backspace; hideCompleted/showArchived state; AlertDialog host
```

Each unit stays focused: `mutations.rs` is the only place that calls `pond-core` ops + rebuilds the snapshot (tested with a tempdir store); `commands.rs` wrappers contain no logic beyond arg → `mutations::` call → string error; `api/` is the only `invoke` site; `state/` is pure (tested); components render and dispatch.

---

## Task 1: Managed store + `get_snapshot` migration + `mutations.rs` scaffold + `create_item`

Establishes the seam every later backend task copies: managed `TaskStore`, `mutations::<op>(store, …) -> Result<SnapshotDto>`, a `#[tauri::command]` wrapper taking `State<TaskStore>`, and registration.

**Files:**
- Create: `src-tauri/src/mutations.rs`
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Write a failing test for `mutations::create_item`**

Create `src-tauri/src/mutations.rs`:

```rust
use crate::commands::build_snapshot;
use crate::dto::SnapshotDto;
use pond_core::{Result, TaskStatus, TaskStore, DEFAULT_COLLECTION};

/// Create a new empty Draft (title typed in the editor). `collection` is the
/// target collection api-name; `None`/empty falls back to the default collection.
pub fn create_item(store: &TaskStore, collection: Option<&str>) -> Result<SnapshotDto> {
    let target = collection.filter(|c| !c.is_empty()).unwrap_or(DEFAULT_COLLECTION);
    store.add("", target, None, true, TaskStatus::Draft)?;
    build_snapshot(store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn store() -> (tempfile::TempDir, TaskStore) {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        (dir, store)
    }

    #[test]
    fn create_item_adds_a_draft() {
        let (_dir, store) = store();
        let snap = create_item(&store, None).unwrap();
        assert_eq!(snap.items.len(), 1);
        assert_eq!(snap.items[0].status, TaskStatus::Draft);
        assert_eq!(snap.items[0].title, "");
        assert_eq!(snap.items[0].collection, "Inbox");
    }

    #[test]
    fn create_item_honors_explicit_collection() {
        let (_dir, store) = store();
        let snap = create_item(&store, Some("Work/Docs")).unwrap();
        assert_eq!(snap.items.len(), 1);
        assert_eq!(snap.items[0].collection, "Work/Docs");
    }
}
```

Note: `build_snapshot` must be `pub` (it already is) and importable from `crate::commands`. Add `mod mutations;` in `main.rs` (Step 4) so the test compiles. Run:

```bash
cargo test -p pond-tauri mutations
```
Expected: **compile error** (`mod mutations;` not yet declared) — declare it in Step 4 first if you prefer red-via-assert; either way this is the failing state.

- [ ] **Step 2: Migrate `get_snapshot` to the managed store + add the `create_item` wrapper**

Edit `src-tauri/src/commands.rs`. Replace the `get_snapshot` command and add the wrapper. The file becomes:

```rust
use crate::dto::{CollectionGroupSummaryDto, CollectionSummaryDto, SnapshotDto};
use crate::mutations;
use pond_core::{Result, TaskStore};
use tauri::State;

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
pub fn get_snapshot(store: State<TaskStore>) -> std::result::Result<SnapshotDto, String> {
    build_snapshot(&store).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_item(
    store: State<TaskStore>,
    collection: Option<String>,
) -> std::result::Result<SnapshotDto, String> {
    mutations::create_item(&store, collection.as_deref()).map_err(|e| e.to_string())
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
        store
            .add("Ship it", "Work/Docs", None, false, TaskStatus::Ready)
            .unwrap();

        let snap = build_snapshot(&store).unwrap();
        assert_eq!(snap.items.len(), 1);
        assert_eq!(snap.items[0].title, "Ship it");
        assert!(snap.collections.iter().any(|c| c.name == "Work/Docs"));
        assert!(snap.groups.iter().any(|g| g.name == "Work"));
    }
}
```

- [ ] **Step 3: Wire the managed store + register both commands in `main.rs`**

Edit `src-tauri/src/main.rs`: add `mod mutations;`, manage a `TaskStore::open_default()`, and register both commands. The file becomes:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod dto;
mod mutations;
mod watcher;

use std::time::Duration;
use tauri::{Emitter, Manager};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::create_item,
        ])
        .setup(|app| {
            app.manage(pond_core::TaskStore::open_default());

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
                        app.manage(std::sync::Mutex::new(w));
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

- [ ] **Step 4: Run the test (now passing) + gate**

```bash
cargo test -p pond-tauri
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: `mutations::tests` (2) + `commands::tests` (1) + existing `dto`/`watcher` tests pass; fmt/clippy clean.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/mutations.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(tauri): managed store + mutations seam + create_item

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 2: Item commands (`update_item`, `set_status`, `move_item`, `delete_item`, `delete_items`)

**Files:**
- Modify: `src-tauri/src/mutations.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Add failing tests for the five item mutations**

Append to `mod tests` in `src-tauri/src/mutations.rs` (the `use super::*;` / `tempdir` / `store()` helper from Task 1 already cover these):

```rust
    fn seed(store: &TaskStore, title: &str) -> pond_core::TaskItem {
        store.add(title, "Inbox", None, false, TaskStatus::Ready).unwrap()
    }

    #[test]
    fn update_item_changes_title_and_status() {
        let (_dir, store) = store();
        let item = seed(&store, "old");
        let snap = update_item(&store, &item.id, Some("new"), None, Some(TaskStatus::OnHold), None).unwrap();
        let got = &snap.items[0];
        assert_eq!(got.title, "new");
        assert_eq!(got.status, TaskStatus::OnHold);
    }

    #[test]
    fn update_item_if_current_skips_stale() {
        let (_dir, store) = store();
        let item = seed(&store, "old");
        store.update(&item.id, Some("changed-out-of-band"), None, None).unwrap();
        // `item` is now stale; the guarded update must be a no-op.
        let snap = update_item(&store, &item.id, Some("ignored"), None, None, Some(item.clone())).unwrap();
        assert_eq!(snap.items[0].title, "changed-out-of-band");
    }

    #[test]
    fn set_status_single_and_if_current() {
        let (_dir, store) = store();
        let item = seed(&store, "t");
        let snap = set_status(&store, TaskStatus::Completed, &item.id, None).unwrap();
        assert_eq!(snap.items[0].status, TaskStatus::Completed);

        let current = snap.items[0].clone();
        let snap = set_status(&store, TaskStatus::Ready, &current.id, Some(current.clone())).unwrap();
        assert_eq!(snap.items[0].status, TaskStatus::Ready);
    }

    #[test]
    fn move_item_changes_collection() {
        let (_dir, store) = store();
        let item = seed(&store, "t");
        let snap = move_item(&store, &item.id, "Work/Docs").unwrap();
        assert_eq!(snap.items[0].collection, "Work/Docs");
    }

    #[test]
    fn delete_item_and_delete_items() {
        let (_dir, store) = store();
        let a = seed(&store, "a");
        let b = seed(&store, "b");
        let snap = delete_item(&store, &a.id).unwrap();
        assert_eq!(snap.items.len(), 1);
        let snap = delete_items(&store, &[b.id.clone()]).unwrap();
        assert_eq!(snap.items.len(), 0);
    }
```

Run `cargo test -p pond-tauri mutations` → **fails to compile** (functions undefined).

- [ ] **Step 2: Implement the five mutation functions**

Append to `src-tauri/src/mutations.rs` (above `#[cfg(test)]`). `TaskItem` is needed — extend the top `use` to `use pond_core::{Result, TaskItem, TaskStatus, TaskStore, DEFAULT_COLLECTION};`:

```rust
pub fn update_item(
    store: &TaskStore,
    id: &str,
    title: Option<&str>,
    collection: Option<&str>,
    status: Option<TaskStatus>,
    if_current: Option<TaskItem>,
) -> Result<SnapshotDto> {
    match if_current {
        Some(expected) => {
            store.update_if_current(id, title, collection, status, &expected)?;
        }
        None => {
            store.update(id, title, collection, status)?;
        }
    }
    build_snapshot(store)
}

pub fn set_status(
    store: &TaskStore,
    status: TaskStatus,
    id: &str,
    if_current: Option<TaskItem>,
) -> Result<SnapshotDto> {
    match if_current {
        Some(expected) => {
            store.set_status_if_current(status, id, &expected)?;
        }
        None => {
            store.set_status(status, &[id.to_string()], None)?;
        }
    }
    build_snapshot(store)
}

pub fn move_item(store: &TaskStore, id: &str, collection: &str) -> Result<SnapshotDto> {
    store.move_item(id, collection)?;
    build_snapshot(store)
}

pub fn delete_item(store: &TaskStore, id: &str) -> Result<SnapshotDto> {
    store.delete(id)?;
    build_snapshot(store)
}

pub fn delete_items(store: &TaskStore, ids: &[String]) -> Result<SnapshotDto> {
    store.delete_many(ids, None)?;
    build_snapshot(store)
}
```

- [ ] **Step 3: Add the command wrappers**

Append to `src-tauri/src/commands.rs` (before `#[cfg(test)]`). Add `use pond_core::{Result, TaskItem, TaskStatus, TaskStore};` items as needed at the top (`TaskItem`, `TaskStatus`):

```rust
#[tauri::command]
pub fn update_item(
    store: State<TaskStore>,
    id: String,
    title: Option<String>,
    collection: Option<String>,
    status: Option<TaskStatus>,
    if_current: Option<TaskItem>,
) -> std::result::Result<SnapshotDto, String> {
    mutations::update_item(
        &store,
        &id,
        title.as_deref(),
        collection.as_deref(),
        status,
        if_current,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_status(
    store: State<TaskStore>,
    status: TaskStatus,
    id: String,
    if_current: Option<TaskItem>,
) -> std::result::Result<SnapshotDto, String> {
    mutations::set_status(&store, status, &id, if_current).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_item(
    store: State<TaskStore>,
    id: String,
    collection: String,
) -> std::result::Result<SnapshotDto, String> {
    mutations::move_item(&store, &id, &collection).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_item(store: State<TaskStore>, id: String) -> std::result::Result<SnapshotDto, String> {
    mutations::delete_item(&store, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_items(
    store: State<TaskStore>,
    ids: Vec<String>,
) -> std::result::Result<SnapshotDto, String> {
    mutations::delete_items(&store, &ids).map_err(|e| e.to_string())
}
```

Tauri passes camelCase JS keys to snake_case Rust params automatically; `if_current` arrives as `ifCurrent` from the frontend. `TaskItem` deserializes from the JS object the frontend already holds (it mirrors the DTO). `status` deserializes from the kebab-case string.

- [ ] **Step 4: Register in `main.rs`**

Extend `generate_handler!` to add `commands::update_item, commands::set_status, commands::move_item, commands::delete_item, commands::delete_items,`.

- [ ] **Step 5: Test + gate**

```bash
cargo test -p pond-tauri
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
```
Expected: green/clean.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/mutations.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(tauri): item mutation commands (update/status/move/delete)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 3: Note commands (`add_note`, `update_note`, `delete_note`)

`add_note` and `delete_note` accept optional `ifCurrent`; **`update_note` does not** (no `_if_current` variant in `pond-core` — see Divergences).

**Files:**
- Modify: `src-tauri/src/mutations.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Failing tests**

Append to `mod tests` in `mutations.rs`:

```rust
    #[test]
    fn add_update_delete_note() {
        let (_dir, store) = store();
        let item = seed(&store, "t");
        let snap = add_note(&store, &item.id, "first body", None).unwrap();
        assert_eq!(snap.items[0].note.as_ref().unwrap().body, "first body");

        let snap = update_note(&store, &item.id, "second body").unwrap();
        assert_eq!(snap.items[0].note.as_ref().unwrap().body, "second body");

        let snap = delete_note(&store, &item.id, None).unwrap();
        assert!(snap.items[0].note.is_none());
    }

    #[test]
    fn add_note_if_current_skips_stale() {
        let (_dir, store) = store();
        let item = seed(&store, "t");
        store.set_status(TaskStatus::OnHold, &[item.id.clone()], None).unwrap();
        // `item` stale → guarded add is a no-op, note stays absent.
        let snap = add_note(&store, &item.id, "ignored", Some(item.clone())).unwrap();
        assert!(snap.items[0].note.is_none());
    }
```

Run `cargo test -p pond-tauri mutations` → fails (undefined fns).

- [ ] **Step 2: Implement**

Append to `mutations.rs`:

```rust
pub fn add_note(
    store: &TaskStore,
    id: &str,
    body: &str,
    if_current: Option<TaskItem>,
) -> Result<SnapshotDto> {
    match if_current {
        Some(expected) => {
            store.add_note_if_current(id, body, &expected)?;
        }
        None => {
            store.add_note(id, body)?;
        }
    }
    build_snapshot(store)
}

pub fn update_note(store: &TaskStore, id: &str, body: &str) -> Result<SnapshotDto> {
    store.update_note(id, body)?;
    build_snapshot(store)
}

pub fn delete_note(
    store: &TaskStore,
    id: &str,
    if_current: Option<TaskItem>,
) -> Result<SnapshotDto> {
    match if_current {
        Some(expected) => {
            store.delete_note_if_current(id, &expected)?;
        }
        None => {
            store.delete_note(id)?;
        }
    }
    build_snapshot(store)
}
```

- [ ] **Step 3: Command wrappers**

Append to `commands.rs`:

```rust
#[tauri::command]
pub fn add_note(
    store: State<TaskStore>,
    id: String,
    body: String,
    if_current: Option<TaskItem>,
) -> std::result::Result<SnapshotDto, String> {
    mutations::add_note(&store, &id, &body, if_current).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_note(
    store: State<TaskStore>,
    id: String,
    body: String,
) -> std::result::Result<SnapshotDto, String> {
    mutations::update_note(&store, &id, &body).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_note(
    store: State<TaskStore>,
    id: String,
    if_current: Option<TaskItem>,
) -> std::result::Result<SnapshotDto, String> {
    mutations::delete_note(&store, &id, if_current).map_err(|e| e.to_string())
}
```

- [ ] **Step 4: Register** `commands::add_note, commands::update_note, commands::delete_note,` in `main.rs`.

- [ ] **Step 5: Test + gate**

```bash
cargo test -p pond-tauri && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/mutations.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(tauri): note mutation commands (add/update/delete)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 4: `merge_item` / `split_item` commands

Real signatures: `merge_item(id, into_previous, title) -> Result<Option<TaskItem>>` (returns `Ok(None)` when the previous row is not Draft|Ready or already has a note, or `source == previous`); `split_item(id, first_title, second_title, requested_second_id: Option) -> Result<TaskItem>` (errors `InvalidTitle` if either trimmed title is empty; promotes a Draft source to Ready; copies the source note to the new second item). The frontend supplies `secondId = None` so the store mints it (see InlineEditor, Task 11).

**Files:**
- Modify: `src-tauri/src/mutations.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Failing tests**

Append to `mod tests`:

```rust
    #[test]
    fn merge_item_appends_into_previous_and_removes_source() {
        let (_dir, store) = store();
        let prev = store.add("Hello ", "Inbox", None, false, TaskStatus::Ready).unwrap();
        let src = store.add("World", "Inbox", None, false, TaskStatus::Ready).unwrap();
        let snap = merge_item(&store, &src.id, &prev.id, "World").unwrap();
        assert_eq!(snap.items.len(), 1);
        assert_eq!(snap.items[0].id, prev.id);
        assert_eq!(snap.items[0].title, "Hello World");
    }

    #[test]
    fn split_item_creates_a_second_task() {
        let (_dir, store) = store();
        let item = store.add("alpha beta", "Inbox", None, false, TaskStatus::Ready).unwrap();
        let snap = split_item(&store, &item.id, "alpha", "beta", None).unwrap();
        assert_eq!(snap.items.len(), 2);
        let titles: Vec<&str> = snap.items.iter().map(|i| i.title.as_str()).collect();
        assert!(titles.contains(&"alpha"));
        assert!(titles.contains(&"beta"));
    }
```

Run `cargo test -p pond-tauri mutations` → fails.

- [ ] **Step 2: Implement**

Append to `mutations.rs`:

```rust
pub fn merge_item(
    store: &TaskStore,
    id: &str,
    into_previous: &str,
    title: &str,
) -> Result<SnapshotDto> {
    store.merge_item(id, into_previous, title)?;
    build_snapshot(store)
}

pub fn split_item(
    store: &TaskStore,
    id: &str,
    first_title: &str,
    second_title: &str,
    second_id: Option<&str>,
) -> Result<SnapshotDto> {
    store.split_item(id, first_title, second_title, second_id)?;
    build_snapshot(store)
}
```

- [ ] **Step 3: Command wrappers**

Append to `commands.rs`:

```rust
#[tauri::command]
pub fn merge_item(
    store: State<TaskStore>,
    id: String,
    into_previous: String,
    title: String,
) -> std::result::Result<SnapshotDto, String> {
    mutations::merge_item(&store, &id, &into_previous, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn split_item(
    store: State<TaskStore>,
    id: String,
    first_title: String,
    second_title: String,
    second_id: Option<String>,
) -> std::result::Result<SnapshotDto, String> {
    mutations::split_item(&store, &id, &first_title, &second_title, second_id.as_deref())
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 4: Register** `commands::merge_item, commands::split_item,` in `main.rs`.

- [ ] **Step 5: Test + gate**

```bash
cargo test -p pond-tauri && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/mutations.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(tauri): merge_item and split_item commands

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 5: Collection commands

`create_collection`, `rename_collection`, `set_collection_color`, `set_collection_archived`, `move_collection`, `clear_items`, `delete_collection`. `create_collection(name, group?)` falls back to `DEFAULT_GROUP`.

**Files:**
- Modify: `src-tauri/src/mutations.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Failing tests**

Append to `mod tests` (add `use pond_core::CollectionColor;` to the test module's top `use` block):

```rust
    #[test]
    fn collection_lifecycle() {
        let (_dir, store) = store();
        // create
        let snap = create_collection(&store, "Errands", None).unwrap();
        assert!(snap.collections.iter().any(|c| c.name == "Errands"));
        // color
        let snap = set_collection_color(&store, "Errands", CollectionColor::Blue).unwrap();
        assert_eq!(snap.collections.iter().find(|c| c.name == "Errands").unwrap().color, CollectionColor::Blue);
        // archive
        let snap = set_collection_archived(&store, "Errands", true).unwrap();
        assert!(snap.collections.iter().find(|c| c.name == "Errands").unwrap().is_archived);
        // rename
        let snap = rename_collection(&store, "Errands", "Tasks").unwrap();
        assert!(snap.collections.iter().any(|c| c.name == "Tasks"));
        assert!(!snap.collections.iter().any(|c| c.name == "Errands"));
        // move to a group (api-name becomes "Work/Tasks")
        let snap = move_collection(&store, "Tasks", "Work").unwrap();
        assert!(snap.collections.iter().any(|c| c.name == "Work/Tasks"));
        // delete
        let snap = delete_collection(&store, "Work/Tasks").unwrap();
        assert!(!snap.collections.iter().any(|c| c.name == "Work/Tasks"));
    }

    #[test]
    fn clear_items_removes_completed_only() {
        let (_dir, store) = store();
        store.create_collection("Box", DEFAULT_GROUP).unwrap();
        store.add("keep", "Box", None, false, TaskStatus::Ready).unwrap();
        store.add("drop", "Box", None, false, TaskStatus::Completed).unwrap();
        let snap = clear_items(&store, "Box", true).unwrap();
        assert_eq!(snap.items.iter().filter(|i| i.collection == "Box").count(), 1);
        assert_eq!(snap.items[0].title, "keep");
    }
```

Run → fails.

- [ ] **Step 2: Implement**

Append to `mutations.rs` (extend top `use` with `CollectionColor` and `DEFAULT_GROUP`: `use pond_core::{CollectionColor, Result, TaskItem, TaskStatus, TaskStore, DEFAULT_COLLECTION, DEFAULT_GROUP};`):

```rust
pub fn create_collection(
    store: &TaskStore,
    name: &str,
    group: Option<&str>,
) -> Result<SnapshotDto> {
    let group = group.filter(|g| !g.is_empty()).unwrap_or(DEFAULT_GROUP);
    store.create_collection(name, group)?;
    build_snapshot(store)
}

pub fn rename_collection(store: &TaskStore, old: &str, new: &str) -> Result<SnapshotDto> {
    store.rename_collection(old, new)?;
    build_snapshot(store)
}

pub fn set_collection_color(
    store: &TaskStore,
    name: &str,
    color: CollectionColor,
) -> Result<SnapshotDto> {
    store.set_collection_color(name, color)?;
    build_snapshot(store)
}

pub fn set_collection_archived(
    store: &TaskStore,
    name: &str,
    is_archived: bool,
) -> Result<SnapshotDto> {
    store.set_collection_archived(name, is_archived)?;
    build_snapshot(store)
}

pub fn move_collection(store: &TaskStore, name: &str, group: &str) -> Result<SnapshotDto> {
    store.move_collection(name, group)?;
    build_snapshot(store)
}

pub fn clear_items(
    store: &TaskStore,
    name: &str,
    completed_only: bool,
) -> Result<SnapshotDto> {
    store.clear_items(name, completed_only)?;
    build_snapshot(store)
}

pub fn delete_collection(store: &TaskStore, name: &str) -> Result<SnapshotDto> {
    store.delete_collection(name)?;
    build_snapshot(store)
}
```

- [ ] **Step 3: Command wrappers**

Append to `commands.rs` (add `CollectionColor` to the top `use pond_core::{...}`):

```rust
#[tauri::command]
pub fn create_collection(
    store: State<TaskStore>,
    name: String,
    group: Option<String>,
) -> std::result::Result<SnapshotDto, String> {
    mutations::create_collection(&store, &name, group.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_collection(
    store: State<TaskStore>,
    old: String,
    new: String,
) -> std::result::Result<SnapshotDto, String> {
    mutations::rename_collection(&store, &old, &new).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_collection_color(
    store: State<TaskStore>,
    name: String,
    color: CollectionColor,
) -> std::result::Result<SnapshotDto, String> {
    mutations::set_collection_color(&store, &name, color).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_collection_archived(
    store: State<TaskStore>,
    name: String,
    is_archived: bool,
) -> std::result::Result<SnapshotDto, String> {
    mutations::set_collection_archived(&store, &name, is_archived).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_collection(
    store: State<TaskStore>,
    name: String,
    group: String,
) -> std::result::Result<SnapshotDto, String> {
    mutations::move_collection(&store, &name, &group).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_items(
    store: State<TaskStore>,
    name: String,
    completed_only: bool,
) -> std::result::Result<SnapshotDto, String> {
    mutations::clear_items(&store, &name, completed_only).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_collection(
    store: State<TaskStore>,
    name: String,
) -> std::result::Result<SnapshotDto, String> {
    mutations::delete_collection(&store, &name).map_err(|e| e.to_string())
}
```

- [ ] **Step 4: Register** all seven (`commands::create_collection, … commands::delete_collection,`) in `main.rs`.

- [ ] **Step 5: Test + gate**

```bash
cargo test -p pond-tauri && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/mutations.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(tauri): collection mutation commands

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 6: Group commands (`create_group`, `rename_group`, `delete_group`)

**Files:**
- Modify: `src-tauri/src/mutations.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Failing tests**

Append to `mod tests`:

```rust
    #[test]
    fn group_lifecycle() {
        let (_dir, store) = store();
        let snap = create_group(&store, "Personal").unwrap();
        assert!(snap.groups.iter().any(|g| g.name == "Personal"));
        let snap = rename_group(&store, "Personal", "Home").unwrap();
        assert!(snap.groups.iter().any(|g| g.name == "Home"));
        assert!(!snap.groups.iter().any(|g| g.name == "Personal"));
        let snap = delete_group(&store, "Home").unwrap();
        assert!(!snap.groups.iter().any(|g| g.name == "Home"));
    }
```

Run → fails.

- [ ] **Step 2: Implement**

Append to `mutations.rs`:

```rust
pub fn create_group(store: &TaskStore, name: &str) -> Result<SnapshotDto> {
    store.create_group(name)?;
    build_snapshot(store)
}

pub fn rename_group(store: &TaskStore, old: &str, new: &str) -> Result<SnapshotDto> {
    store.rename_group(old, new)?;
    build_snapshot(store)
}

pub fn delete_group(store: &TaskStore, name: &str) -> Result<SnapshotDto> {
    store.delete_group(name)?;
    build_snapshot(store)
}
```

- [ ] **Step 3: Command wrappers**

Append to `commands.rs`:

```rust
#[tauri::command]
pub fn create_group(store: State<TaskStore>, name: String) -> std::result::Result<SnapshotDto, String> {
    mutations::create_group(&store, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_group(
    store: State<TaskStore>,
    old: String,
    new: String,
) -> std::result::Result<SnapshotDto, String> {
    mutations::rename_group(&store, &old, &new).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_group(store: State<TaskStore>, name: String) -> std::result::Result<SnapshotDto, String> {
    mutations::delete_group(&store, &name).map_err(|e| e.to_string())
}
```

- [ ] **Step 4: Register** `commands::create_group, commands::rename_group, commands::delete_group,` in `main.rs`.

- [ ] **Step 5: Test + gate**

```bash
cargo test -p pond-tauri && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/mutations.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(tauri): group mutation commands

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 7: Typed frontend client wrappers + arg types

One typed wrapper per command; each `invoke<Snapshot>(name, args)`. Extend `client.test.ts` (mocked invoke) for a couple of wrappers (right command name + camelCase args).

**Files:**
- Modify: `src/api/types.ts`, `src/api/client.ts`, `src/api/client.test.ts`

- [ ] **Step 1: Add a failing test for two wrappers**

Append to `src/api/client.test.ts` (inside `describe`):

```ts
  it("createItem invokes create_item with the collection arg", async () => {
    invokeMock.mockResolvedValue({ items: [], collections: [], groups: [] });
    await createItem("Work/Docs");
    expect(invokeMock).toHaveBeenCalledWith("create_item", { collection: "Work/Docs" });
  });

  it("setStatus invokes set_status with id/status/ifCurrent", async () => {
    invokeMock.mockResolvedValue({ items: [], collections: [], groups: [] });
    const item = { id: "00000001" } as unknown as TaskItem;
    await setStatus("completed", "00000001", item);
    expect(invokeMock).toHaveBeenCalledWith("set_status", {
      status: "completed",
      id: "00000001",
      ifCurrent: item,
    });
  });
```

Extend the import line at top: `import { getSnapshot, onStoreChanged, createItem, setStatus } from "./client";` and add `import type { TaskItem } from "./types";`. Run:

```bash
npx vitest run src/api/client.test.ts
```
Expected: **fail** (`createItem`/`setStatus` not exported).

- [ ] **Step 2: Add arg helper types to `types.ts`**

Append to `src/api/types.ts`:

```ts
export type CollectionColorName = CollectionColor;
```

(The DTOs are otherwise unchanged. `TaskStatus`/`CollectionColor`/`TaskItem` already exist.)

- [ ] **Step 3: Implement the wrappers in `client.ts`**

Replace `src/api/client.ts` with (keeping the existing two functions, adding the rest):

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CollectionColor, Snapshot, TaskItem, TaskStatus } from "./types";

export function getSnapshot(): Promise<Snapshot> {
  return invoke<Snapshot>("get_snapshot");
}

export function onStoreChanged(callback: () => void): Promise<UnlistenFn> {
  return listen("store-changed", () => callback());
}

// --- Items ---
export function createItem(collection?: string): Promise<Snapshot> {
  return invoke<Snapshot>("create_item", { collection: collection ?? null });
}

export function updateItem(
  id: string,
  fields: { title?: string; collection?: string; status?: TaskStatus },
  ifCurrent?: TaskItem,
): Promise<Snapshot> {
  return invoke<Snapshot>("update_item", {
    id,
    title: fields.title ?? null,
    collection: fields.collection ?? null,
    status: fields.status ?? null,
    ifCurrent: ifCurrent ?? null,
  });
}

export function setStatus(status: TaskStatus, id: string, ifCurrent?: TaskItem): Promise<Snapshot> {
  return invoke<Snapshot>("set_status", { status, id, ifCurrent: ifCurrent ?? null });
}

export function moveItem(id: string, collection: string): Promise<Snapshot> {
  return invoke<Snapshot>("move_item", { id, collection });
}

export function deleteItem(id: string): Promise<Snapshot> {
  return invoke<Snapshot>("delete_item", { id });
}

export function deleteItems(ids: string[]): Promise<Snapshot> {
  return invoke<Snapshot>("delete_items", { ids });
}

// --- Notes ---
export function addNote(id: string, body: string, ifCurrent?: TaskItem): Promise<Snapshot> {
  return invoke<Snapshot>("add_note", { id, body, ifCurrent: ifCurrent ?? null });
}

export function updateNote(id: string, body: string): Promise<Snapshot> {
  return invoke<Snapshot>("update_note", { id, body });
}

export function deleteNote(id: string, ifCurrent?: TaskItem): Promise<Snapshot> {
  return invoke<Snapshot>("delete_note", { id, ifCurrent: ifCurrent ?? null });
}

// --- Merge / split ---
export function mergeItem(id: string, intoPrevious: string, title: string): Promise<Snapshot> {
  return invoke<Snapshot>("merge_item", { id, intoPrevious, title });
}

export function splitItem(
  id: string,
  firstTitle: string,
  secondTitle: string,
  secondId?: string,
): Promise<Snapshot> {
  return invoke<Snapshot>("split_item", { id, firstTitle, secondTitle, secondId: secondId ?? null });
}

// --- Collections ---
export function createCollection(name: string, group?: string): Promise<Snapshot> {
  return invoke<Snapshot>("create_collection", { name, group: group ?? null });
}

export function renameCollection(oldName: string, newName: string): Promise<Snapshot> {
  return invoke<Snapshot>("rename_collection", { old: oldName, new: newName });
}

export function setCollectionColor(name: string, color: CollectionColor): Promise<Snapshot> {
  return invoke<Snapshot>("set_collection_color", { name, color });
}

export function setCollectionArchived(name: string, isArchived: boolean): Promise<Snapshot> {
  return invoke<Snapshot>("set_collection_archived", { name, isArchived });
}

export function moveCollection(name: string, group: string): Promise<Snapshot> {
  return invoke<Snapshot>("move_collection", { name, group });
}

export function clearItems(name: string, completedOnly: boolean): Promise<Snapshot> {
  return invoke<Snapshot>("clear_items", { name, completedOnly });
}

export function deleteCollection(name: string): Promise<Snapshot> {
  return invoke<Snapshot>("delete_collection", { name });
}

// --- Groups ---
export function createGroup(name: string): Promise<Snapshot> {
  return invoke<Snapshot>("create_group", { name });
}

export function renameGroup(oldName: string, newName: string): Promise<Snapshot> {
  return invoke<Snapshot>("rename_group", { old: oldName, new: newName });
}

export function deleteGroup(name: string): Promise<Snapshot> {
  return invoke<Snapshot>("delete_group", { name });
}
```

Note the test expects `create_item` called with `{ collection: "Work/Docs" }`; the `?? null` for an omitted arg produces `{ collection: null }`, which the assertion in Step 1 (passing `"Work/Docs"`) satisfies. (Tauri treats `null` and an omitted key identically for `Option<_>` params.)

- [ ] **Step 2.5 (if needed): align the test's no-arg expectation.** If you also assert the no-arg `createItem()` call shape, expect `{ collection: null }`.

- [ ] **Step 3: Run the test (passing) + gate**

```bash
npx vitest run src/api/client.test.ts
npx tsc --noEmit && npm run build
```
Expected: green/clean.

- [ ] **Step 4: Commit**

```bash
git add src/api/client.ts src/api/types.ts src/api/client.test.ts
git commit -m "feat(ui): typed mutation client wrappers

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 8: `view.ts` (hideCompleted/showArchived) + NEW `status.ts`

Add `hideCompleted` filtering and `showArchived` to selectors; add a pure `leadingStatusClickTarget(status)` (ready→completed, in-progress→completed, else→ready) and a right-click target (`"draft"`). Parity source: `TaskViewSupport.swift` `leadingStatusClickTarget` (ready→.completed, inProgress→.completed, default→.ready).

**Files:**
- Modify: `src/state/view.ts`, `src/state/view.test.ts`
- Create: `src/state/status.ts`, `src/state/status.test.ts`

- [ ] **Step 1: Failing test — status targets**

Create `src/state/status.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { leadingStatusClickTarget, rightClickStatusTarget } from "./status";

describe("leadingStatusClickTarget", () => {
  it("ready and in-progress advance to completed", () => {
    expect(leadingStatusClickTarget("ready")).toBe("completed");
    expect(leadingStatusClickTarget("in-progress")).toBe("completed");
  });
  it("everything else advances to ready", () => {
    expect(leadingStatusClickTarget("draft")).toBe("ready");
    expect(leadingStatusClickTarget("completed")).toBe("ready");
    expect(leadingStatusClickTarget("on-hold")).toBe("ready");
    expect(leadingStatusClickTarget("rejected")).toBe("ready");
    expect(leadingStatusClickTarget("aborted")).toBe("ready");
  });
  it("right-click always targets draft", () => {
    expect(rightClickStatusTarget("ready")).toBe("draft");
    expect(rightClickStatusTarget("completed")).toBe("draft");
  });
});
```

- [ ] **Step 2: Failing test — extended view selectors**

Edit `src/state/view.test.ts`: the `ViewState` literals gain `hideCompleted`/`showArchived`. Append a new describe and update existing literals to include the new fields (they default `false`). Add:

```ts
import { sidebarGroups } from "./view";

describe("hideCompleted + showArchived", () => {
  it("hideCompleted removes completed items from visibleItems", () => {
    const s = snap();
    const all = visibleItems(s, { selected: ALL_COLLECTION, search: "", incompleteOnly: false, hideCompleted: false, showArchived: false });
    expect(all.length).toBe(3);
    const hidden = visibleItems(s, { selected: ALL_COLLECTION, search: "", incompleteOnly: false, hideCompleted: true, showArchived: false });
    expect(hidden.map((i) => i.id)).toEqual(["00000001", "00000003"]);
  });

  it("sidebarGroups hides archived collections unless showArchived", () => {
    const s: Snapshot = {
      items: [],
      collections: [],
      groups: [
        { name: "Work", collections: [
          { name: "Work/A", displayName: "A", groupName: "Work", totalCount: 0, incompleteCount: 0, color: "gray", isArchived: false },
          { name: "Work/B", displayName: "B", groupName: "Work", totalCount: 0, incompleteCount: 0, color: "gray", isArchived: true },
        ]},
      ],
    };
    expect(sidebarGroups(s, false)[0].collections.map((c) => c.name)).toEqual(["Work/A"]);
    expect(sidebarGroups(s, true)[0].collections.map((c) => c.name)).toEqual(["Work/A", "Work/B"]);
  });
});
```

Also update the three existing `visibleItems(...)` calls in this file to include `hideCompleted: false, showArchived: false` in their `ViewState` literals (TypeScript will otherwise error once `ViewState` requires them).

Run `npx vitest run src/state` → **fails** (status.ts missing; ViewState shape mismatch).

- [ ] **Step 3: Create `src/state/status.ts`**

```ts
import type { TaskStatus } from "../api/types";

/**
 * Left-click on the leading status dot advances the status.
 * Mirrors Swift `TaskStatus.leadingStatusClickTarget` (TaskViewSupport.swift):
 * ready -> completed, in-progress -> completed, everything else -> ready.
 */
export function leadingStatusClickTarget(status: TaskStatus): TaskStatus {
  switch (status) {
    case "ready":
    case "in-progress":
      return "completed";
    default:
      return "ready";
  }
}

/** Right-click on the leading status dot sets the task back to draft. */
export function rightClickStatusTarget(_status: TaskStatus): TaskStatus {
  return "draft";
}
```

- [ ] **Step 4: Extend `src/state/view.ts`**

```ts
import type { CollectionGroupSummary, Snapshot, TaskItem } from "../api/types";

export const ALL_COLLECTION = "__all__";

export interface ViewState {
  selected: string; // ALL_COLLECTION or a collection name
  search: string;
  incompleteOnly: boolean;
  hideCompleted: boolean;
  showArchived: boolean;
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
    const completedHidden = (view.incompleteOnly || view.hideCompleted) && item.status === "completed";
    return collectionMatches && !completedHidden && matchesSearch(item, view.search);
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

(`incompleteOnly` is retained for back-compat; both it and `hideCompleted` hide completed items.)

- [ ] **Step 5: Run tests (passing) + gate**

```bash
npx vitest run src/state
npx tsc --noEmit && npm run build
```
Note: `App.tsx` constructs a `ViewState` literal (Task 1's Phase 3 code) — it will now fail `tsc` until Task 10 adds the new fields. For an isolated green gate here, temporarily add `hideCompleted: false, showArchived: false` to the `useState<ViewState>` initializer in `App.tsx` as part of this task (Task 10 builds on it). Re-run `npx tsc --noEmit && npm run build` → clean.

- [ ] **Step 6: Commit**

```bash
git add src/state/view.ts src/state/view.test.ts src/state/status.ts src/state/status.test.ts src/App.tsx
git commit -m "feat(ui): hideCompleted/showArchived selectors + status targets

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 9: NEW `src/state/editor.ts` — the pure key-handling reducer (highest-risk unit)

A pure reducer mapping `(key, caretAtStart, caretAtEnd, value, field, composing)` → an `EditorIntent` union. The component (Task 11) executes the intent. **Parity source** (`Sources/PondApp/TaskRow.swift`):
- Title `handleTitleKeyDown` (≈ lines 836–905): `escape`→discard; `returnKey`/`keypadEnter` **guarded by `event.isCommandReturnKey`** → confirm (`handleTitleReturn` = move focus down, confirm title); `tab` (plain) → confirm + move to next; `backspace` → `deleteIfEmptyTitleAtStart` then `mergeWithPrevious(item, fieldEditor.string)`; `arrowUp`/`arrowDown` (plain) → move focus between rows.
- `deleteIfEmptyTitleAtStart` (≈ 1110): only fires when **caret at location 0, length 0** and not a modified backspace → calls `mergeWithPrevious`. The Swift split-at-caret lives in `handlePlainTitleReturn` (≈ 1133): caret at end with non-empty prefix → create item below; mid-title with both sides non-empty → `splitTitle`.
- Note `handleNoteKeyDown` (≈ 968): `escape`→discard; `tab`→move to next; empty note on save → delete (`saveNoteIfNeeded(allowsEmptyRemoval: true)`).
- IME: `isComposingTitle`/`isComposingNote` and `hasMarkedText()` suppress Enter/split (`composing=true` ⇒ no Split/Commit on Enter).

This plan follows the **design §5 key map** (the canonical Phase 4 contract), reconciled with the Swift source: **plain Enter → Split-at-caret** (the user-facing Phase 4 binding), **Cmd+Enter → Commit**, **Backspace@start → MergeIntoPrevious**, **Tab → Commit + MoveFocus(down)**, **Esc → Discard**, **↑/↓ → MoveFocus**, empty-on-commit → DeleteEmpty.

**Files:**
- Create: `src/state/editor.ts`, `src/state/editor.test.ts`

- [ ] **Step 1: Write the failing reducer test (every branch)**

Create `src/state/editor.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { reduceKey, type KeyContext } from "./editor";

function ctx(over: Partial<KeyContext>): KeyContext {
  return {
    key: "Enter",
    metaKey: false,
    shiftKey: false,
    caretAtStart: false,
    caretAtEnd: false,
    value: "hello",
    field: "title",
    composing: false,
    ...over,
  };
}

describe("editor reducer — title field", () => {
  it("Enter splits at caret", () => {
    expect(reduceKey(ctx({ key: "Enter", value: "ab", caretAtStart: false, caretAtEnd: false })))
      .toEqual({ type: "Split" });
  });

  it("Enter at end of a non-empty title creates a task below (Split with caretAtEnd)", () => {
    expect(reduceKey(ctx({ key: "Enter", value: "ab", caretAtEnd: true })))
      .toEqual({ type: "Split" });
  });

  it("Enter on an empty title commits (which deletes the empty draft)", () => {
    expect(reduceKey(ctx({ key: "Enter", value: "", caretAtStart: true, caretAtEnd: true })))
      .toEqual({ type: "DeleteEmpty" });
  });

  it("Enter while composing is suppressed (IME)", () => {
    expect(reduceKey(ctx({ key: "Enter", value: "あ", composing: true })))
      .toEqual({ type: "None" });
  });

  it("Cmd+Enter commits a non-empty title", () => {
    expect(reduceKey(ctx({ key: "Enter", metaKey: true, value: "ab" })))
      .toEqual({ type: "Commit" });
  });

  it("Cmd+Enter on an empty title deletes", () => {
    expect(reduceKey(ctx({ key: "Enter", metaKey: true, value: "" })))
      .toEqual({ type: "DeleteEmpty" });
  });

  it("Backspace at caret 0 merges into previous", () => {
    expect(reduceKey(ctx({ key: "Backspace", caretAtStart: true, value: "x" })))
      .toEqual({ type: "MergeIntoPrevious" });
  });

  it("Backspace not at start is a no-op (let the textarea edit)", () => {
    expect(reduceKey(ctx({ key: "Backspace", caretAtStart: false, value: "x" })))
      .toEqual({ type: "None" });
  });

  it("Tab commits and moves focus down", () => {
    expect(reduceKey(ctx({ key: "Tab", value: "ab" })))
      .toEqual({ type: "Commit", thenFocus: "down" });
  });

  it("Tab on empty deletes then moves down", () => {
    expect(reduceKey(ctx({ key: "Tab", value: "" })))
      .toEqual({ type: "DeleteEmpty", thenFocus: "down" });
  });

  it("Escape discards", () => {
    expect(reduceKey(ctx({ key: "Escape" }))).toEqual({ type: "Discard" });
  });

  it("ArrowUp / ArrowDown move focus", () => {
    expect(reduceKey(ctx({ key: "ArrowUp" }))).toEqual({ type: "MoveFocus", dir: "up" });
    expect(reduceKey(ctx({ key: "ArrowDown" }))).toEqual({ type: "MoveFocus", dir: "down" });
  });

  it("any other key is None (textarea handles it)", () => {
    expect(reduceKey(ctx({ key: "a" }))).toEqual({ type: "None" });
  });
});

describe("editor reducer — note field", () => {
  it("Enter (Return) moves focus down", () => {
    expect(reduceKey(ctx({ field: "note", key: "Enter", value: "n" })))
      .toEqual({ type: "Commit", thenFocus: "down" });
  });

  it("Tab moves focus down", () => {
    expect(reduceKey(ctx({ field: "note", key: "Tab", value: "n" })))
      .toEqual({ type: "Commit", thenFocus: "down" });
  });

  it("empty note on commit deletes the note", () => {
    expect(reduceKey(ctx({ field: "note", key: "Enter", value: "" })))
      .toEqual({ type: "DeleteEmpty", thenFocus: "down" });
  });

  it("Escape discards", () => {
    expect(reduceKey(ctx({ field: "note", key: "Escape" }))).toEqual({ type: "Discard" });
  });

  it("Enter while composing is suppressed (IME)", () => {
    expect(reduceKey(ctx({ field: "note", key: "Enter", value: "ん", composing: true })))
      .toEqual({ type: "None" });
  });
});
```

Run `npx vitest run src/state/editor.test.ts` → **fail** (module missing).

- [ ] **Step 2: Implement `src/state/editor.ts`**

```ts
export type FocusDir = "up" | "down";

export type EditorIntent =
  | { type: "Split" }
  | { type: "MergeIntoPrevious" }
  | { type: "Commit"; thenFocus?: FocusDir }
  | { type: "MoveFocus"; dir: FocusDir }
  | { type: "DeleteEmpty"; thenFocus?: FocusDir }
  | { type: "Discard" }
  | { type: "None" };

export interface KeyContext {
  key: string; // KeyboardEvent.key
  metaKey: boolean; // Cmd (macOS)
  shiftKey: boolean;
  caretAtStart: boolean; // selection collapsed at offset 0
  caretAtEnd: boolean; // selection collapsed at end of value
  value: string; // current draft text
  field: "title" | "note";
  composing: boolean; // IME composition in progress
}

function isEmpty(value: string): boolean {
  return value.trim().length === 0;
}

function reduceTitle(c: KeyContext): EditorIntent {
  switch (c.key) {
    case "Escape":
      return { type: "Discard" };
    case "Enter": {
      if (c.composing) return { type: "None" }; // IME commit — never split
      if (c.metaKey) {
        // Cmd+Enter → confirm (Swift handleTitleReturn, guarded by isCommandReturnKey)
        return isEmpty(c.value) ? { type: "DeleteEmpty" } : { type: "Commit" };
      }
      // Plain Enter → split at caret (Swift handlePlainTitleReturn). Empty title → delete the draft.
      return isEmpty(c.value) ? { type: "DeleteEmpty" } : { type: "Split" };
    }
    case "Tab": {
      // Swift: plain Tab confirms title then moves focus down.
      return isEmpty(c.value)
        ? { type: "DeleteEmpty", thenFocus: "down" }
        : { type: "Commit", thenFocus: "down" };
    }
    case "Backspace": {
      // Swift deleteIfEmptyTitleAtStart: only at caret location 0, collapsed.
      if (c.caretAtStart) return { type: "MergeIntoPrevious" };
      return { type: "None" };
    }
    case "ArrowUp":
      return { type: "MoveFocus", dir: "up" };
    case "ArrowDown":
      return { type: "MoveFocus", dir: "down" };
    default:
      return { type: "None" };
  }
}

function reduceNote(c: KeyContext): EditorIntent {
  switch (c.key) {
    case "Escape":
      return { type: "Discard" };
    case "Enter":
    case "Tab": {
      if (c.composing) return { type: "None" }; // IME — do not commit/move
      // Swift handleNoteKeyDown: Return/Tab move focus down; empty note removes it.
      return isEmpty(c.value)
        ? { type: "DeleteEmpty", thenFocus: "down" }
        : { type: "Commit", thenFocus: "down" };
    }
    case "ArrowUp":
      return { type: "MoveFocus", dir: "up" };
    case "ArrowDown":
      return { type: "MoveFocus", dir: "down" };
    default:
      return { type: "None" };
  }
}

/** Pure key → intent. The component executes the intent (calls the client). */
export function reduceKey(c: KeyContext): EditorIntent {
  return c.field === "title" ? reduceTitle(c) : reduceNote(c);
}
```

Note: `caretAtEnd` is part of the context (the component computes it from the DOM selection) and is available to the executor for choosing the `split_item` payload (caret-at-end ⇒ second title would be empty ⇒ the executor falls back to "create empty draft below" via `createItem`, matching Swift `createItemBelowFromTitle`). The reducer itself returns `Split` for both mid-caret and end-caret; the **executor** (Task 11) branches on `caretAtEnd` to pick `split_item` vs `createItem`.

- [ ] **Step 3: Run tests (passing) + gate**

```bash
npx vitest run src/state/editor.test.ts
npx tsc --noEmit && npm run build
```
Expected: all branches green; clean.

- [ ] **Step 4: Commit**

```bash
git add src/state/editor.ts src/state/editor.test.ts
git commit -m "feat(ui): pure inline-editor key reducer

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 10: `App.tsx` — snapshot-from-result wiring, shortcuts, view toggles, AlertDialog host

Replace snapshot from every mutation result; keep `store-changed` refetch for external edits; `Cmd+N` (create_item in selected collection, fallback default) + `Cmd+Backspace` (delete focused); hold `hideCompleted`/`showArchived`; host a single `AlertDialog` for destructive confirms. Parity: `AppModel.swift` `setStatus`/create/`delete` use `ifCurrent`; master spec §6 shortcuts.

> **Radix 3.3.0 latitude:** the `AlertDialog` host below uses the installed `@radix-ui/themes` `AlertDialog` parts (`AlertDialog.Root/Content/Title/Description/Cancel/Action`, controlled via `open`/`onOpenChange`). If a prop/part name differs in 3.3.0, adjust to the installed API; gate is clean tsc+build.

**Files:**
- Modify: `src/App.tsx`
- Create: `src/state/confirm.ts` (tiny shared type for the confirm host)

- [ ] **Step 1: Add the confirm descriptor type**

Create `src/state/confirm.ts`:

```ts
export interface ConfirmRequest {
  title: string;
  description: string;
  confirmLabel: string;
  onConfirm: () => void;
}
```

- [ ] **Step 2: Rewrite `App.tsx`**

```tsx
import { useCallback, useEffect, useRef, useState } from "react";
import { AlertDialog, Button, Flex } from "@radix-ui/themes";
import type { Snapshot } from "./api/types";
import {
  createItem,
  deleteItem,
  getSnapshot,
  onStoreChanged,
} from "./api/client";
import { ALL_COLLECTION, type ViewState } from "./state/view";
import type { ConfirmRequest } from "./state/confirm";
import { Sidebar } from "./components/Sidebar";
import { DetailPane } from "./components/DetailPane";

const EMPTY: Snapshot = { items: [], collections: [], groups: [] };

export function App() {
  const [snapshot, setSnapshot] = useState<Snapshot>(EMPTY);
  const [view, setView] = useState<ViewState>({
    selected: ALL_COLLECTION,
    search: "",
    incompleteOnly: false,
    hideCompleted: false,
    showArchived: false,
  });
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<ConfirmRequest | null>(null);

  // Always-current snapshot for keyboard handlers.
  const snapRef = useRef(snapshot);
  snapRef.current = snapshot;
  const viewRef = useRef(view);
  viewRef.current = view;
  const focusRef = useRef(focusedId);
  focusRef.current = focusedId;

  // Initial load + external (CLI) edits.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    const refresh = () => {
      getSnapshot().then(setSnapshot).catch((e) => console.error(e));
    };
    refresh();
    onStoreChanged(refresh).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Every mutation returns a fresh snapshot; child callbacks call client wrappers
  // and pass the resolved snapshot here.
  const apply = useCallback((next: Snapshot) => setSnapshot(next), []);
  const requestConfirm = useCallback((req: ConfirmRequest) => setConfirm(req), []);

  // Cmd+N (create in selected collection; "All" → default) and Cmd+Backspace (delete focused).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey && (e.key === "n" || e.key === "N")) {
        e.preventDefault();
        const sel = viewRef.current.selected;
        const target = sel === ALL_COLLECTION ? undefined : sel;
        createItem(target)
          .then((snap) => {
            setSnapshot(snap);
            // Focus + edit the newly created (empty) draft: the last item in the target.
            const created = [...snap.items]
              .reverse()
              .find((i) => i.title === "" && i.status === "draft" && (!target || i.collection === target));
            if (created) {
              setFocusedId(created.id);
              setEditingId(created.id);
            }
          })
          .catch((err) => console.error(err));
      } else if (e.metaKey && (e.key === "Backspace" || e.key === "Delete")) {
        const id = focusRef.current;
        if (id) {
          e.preventDefault();
          deleteItem(id).then(setSnapshot).catch((err) => console.error(err));
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <Flex height="100vh">
      <Sidebar
        snapshot={snapshot}
        selected={view.selected}
        showArchived={view.showArchived}
        hideCompleted={view.hideCompleted}
        onSelect={(name) => setView((v) => ({ ...v, selected: name }))}
        onToggleHideCompleted={() => setView((v) => ({ ...v, hideCompleted: !v.hideCompleted }))}
        onToggleShowArchived={() => setView((v) => ({ ...v, showArchived: !v.showArchived }))}
        onSnapshot={apply}
        onRequestConfirm={requestConfirm}
      />
      <DetailPane
        snapshot={snapshot}
        view={view}
        focusedId={focusedId}
        editingId={editingId}
        onSearch={(q) => setView((v) => ({ ...v, search: q }))}
        onFocusItem={setFocusedId}
        onEditItem={setEditingId}
        onSnapshot={apply}
        onRequestConfirm={requestConfirm}
      />

      <AlertDialog.Root open={confirm !== null} onOpenChange={(o) => { if (!o) setConfirm(null); }}>
        <AlertDialog.Content maxWidth="420px">
          <AlertDialog.Title>{confirm?.title ?? ""}</AlertDialog.Title>
          <AlertDialog.Description size="2">{confirm?.description ?? ""}</AlertDialog.Description>
          <Flex gap="3" mt="4" justify="end">
            <AlertDialog.Cancel>
              <Button variant="soft" color="gray">Cancel</Button>
            </AlertDialog.Cancel>
            <AlertDialog.Action>
              <Button
                color="red"
                onClick={() => {
                  confirm?.onConfirm();
                  setConfirm(null);
                }}
              >
                {confirm?.confirmLabel ?? "Delete"}
              </Button>
            </AlertDialog.Action>
          </Flex>
        </AlertDialog.Content>
      </AlertDialog.Root>
    </Flex>
  );
}
```

Critical wiring: `onSnapshot(apply)` is threaded into Sidebar/DetailPane so a child wrapper call like `setStatus(...).then(onSnapshot)` replaces state from the result. The `store-changed` listener stays for external edits only. `focusedId`/`editingId` drive row focus + which row's editor is open.

- [ ] **Step 3: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green. (Sidebar/DetailPane/TaskRow signatures get the new props in Tasks 11–13; if you implement App first, `tsc` will report missing props — implement Tasks 11–13 then re-gate, or land App together with the component prop additions. Recommended order: do Task 11 (InlineEditor) and Task 12/13 prop plumbing, then re-run this gate before committing Task 10. If committing Task 10 alone, add the new props to the three component signatures as no-op pass-throughs to keep `tsc` green, fleshed out in 11–13.)

- [ ] **Step 4: Commit**

```bash
git add src/App.tsx src/state/confirm.ts
git commit -m "feat(ui): snapshot-from-result wiring, Cmd+N/Cmd+Backspace, confirm host

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 11: NEW `src/components/InlineEditor.tsx` — Text⇄TextArea, local draft, IME-safe autosave, intent execution

Click swaps a display `Text` to a Radix `TextArea`; the field holds **local draft** state independent of the snapshot; **500 ms debounced autosave** via the `*_if_current` wrappers, **skipped while composing**; keydown → `reduceKey` (editor.ts) → execute the `EditorIntent`. In-progress/completed **lock title** editing (note still editable). Parity source: `TaskRow.swift` (autosave `Task.sleep(500_000_000)`; `isComposing*` guards; `mergeWithPrevious` precondition; `handlePlainTitleReturn` caret-at-end vs split).

> **Radix 3.3.0 latitude:** uses `TextArea` (controlled `value`/`onChange`, `onKeyDown`, `onBlur`, `onCompositionStart/End`, `autoFocus`) and `Text`. If 3.3.0's `TextArea` prop surface differs, adjust to the installed API; gate is clean tsc+build.

**Files:**
- Create: `src/components/InlineEditor.tsx`

- [ ] **Step 1: Implement `InlineEditor.tsx`**

```tsx
import { useEffect, useRef, useState } from "react";
import { Text, TextArea } from "@radix-ui/themes";
import type { Snapshot, TaskItem } from "../api/types";
import {
  addNote,
  createItem,
  deleteItem,
  deleteNote,
  mergeItem,
  splitItem,
  updateItem,
  updateNote,
} from "../api/client";
import { reduceKey, type EditorIntent, type FocusDir } from "../state/editor";

const AUTOSAVE_MS = 500;

export interface InlineEditorProps {
  item: TaskItem;
  field: "title" | "note";
  /** Previous row's item, for the Backspace-merge precondition (title only). */
  previous?: TaskItem;
  editing: boolean;
  onBeginEdit: () => void;
  onEndEdit: () => void;
  onMoveFocus: (dir: FocusDir) => void;
  onSnapshot: (snap: Snapshot) => void;
}

/** Swift mergeWithPrevious precondition: previous is draft/ready AND has no note. */
function canMergeInto(previous: TaskItem | undefined): previous is TaskItem {
  return (
    !!previous &&
    (previous.status === "draft" || previous.status === "ready") &&
    !previous.note
  );
}

export function InlineEditor({
  item,
  field,
  previous,
  editing,
  onBeginEdit,
  onEndEdit,
  onMoveFocus,
  onSnapshot,
}: InlineEditorProps) {
  const initial = field === "title" ? item.title : (item.note?.body ?? "");
  const [draft, setDraft] = useState(initial);
  const composingRef = useRef(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const ref = useRef<HTMLTextAreaElement | null>(null);

  // Title editing is locked once the task is in-progress or completed (note stays editable).
  const locked = field === "title" && (item.status === "in-progress" || item.status === "completed");

  // Reset the local draft when (re)entering edit mode, so a stale draft never leaks.
  useEffect(() => {
    if (editing) {
      setDraft(field === "title" ? item.title : (item.note?.body ?? ""));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editing]);

  const clearTimer = () => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };

  const scheduleAutosave = (value: string) => {
    clearTimer();
    if (composingRef.current) return; // never autosave mid-IME-composition
    timerRef.current = setTimeout(() => {
      void save(value, { fromAutosave: true });
    }, AUTOSAVE_MS);
  };

  // Persist the draft. ifCurrent = the snapshot item we started from (optimistic concurrency).
  const save = async (value: string, opts?: { fromAutosave?: boolean }) => {
    clearTimer();
    const trimmed = value.trim();
    try {
      if (field === "title") {
        if (trimmed === item.title) return; // unchanged
        const snap = await updateItem(item.id, { title: value }, item);
        onSnapshot(snap);
      } else {
        // note
        if (trimmed.length === 0) {
          if (item.note) onSnapshot(await deleteNote(item.id, item));
          return;
        }
        if (item.note) {
          if (trimmed === item.note.body) return;
          onSnapshot(await updateNote(item.id, value)); // no _if_current variant in core
        } else {
          onSnapshot(await addNote(item.id, value, item));
        }
      }
    } catch (e) {
      console.error(e);
    } finally {
      if (!opts?.fromAutosave) onEndEdit();
    }
  };

  const caretAtStart = () => {
    const el = ref.current;
    return !!el && el.selectionStart === 0 && el.selectionEnd === 0;
  };
  const caretAtEnd = () => {
    const el = ref.current;
    return !!el && el.selectionStart === draft.length && el.selectionEnd === draft.length;
  };

  const execute = async (intent: EditorIntent) => {
    switch (intent.type) {
      case "Split": {
        clearTimer();
        const el = ref.current;
        const caret = el ? el.selectionStart : draft.length;
        const first = draft.slice(0, caret);
        const second = draft.slice(caret);
        if (second.trim().length === 0) {
          // Caret at end → create an empty draft below (Swift createItemBelowFromTitle).
          // First, persist the current (non-empty) title, then create below.
          await updateItem(item.id, { title: first }, item).then(onSnapshot).catch(console.error);
          await createItem(item.collection).then(onSnapshot).catch(console.error);
        } else if (first.trim().length === 0) {
          // No usable first title → no-op (Swift returns true without splitting).
        } else {
          await splitItem(item.id, first, second).then(onSnapshot).catch(console.error);
        }
        onEndEdit();
        break;
      }
      case "MergeIntoPrevious": {
        clearTimer();
        if (canMergeInto(previous)) {
          await mergeItem(item.id, previous.id, draft).then(onSnapshot).catch(console.error);
          onEndEdit();
        }
        // If not mergeable, swallow the Backspace (do nothing) — matches Swift gate.
        break;
      }
      case "Commit": {
        await save(draft);
        if (intent.thenFocus) onMoveFocus(intent.thenFocus);
        break;
      }
      case "DeleteEmpty": {
        clearTimer();
        if (field === "title") {
          await deleteItem(item.id).then(onSnapshot).catch(console.error);
        } else if (item.note) {
          await deleteNote(item.id, item).then(onSnapshot).catch(console.error);
        }
        onEndEdit();
        if (intent.thenFocus) onMoveFocus(intent.thenFocus);
        break;
      }
      case "MoveFocus":
        onEndEdit();
        onMoveFocus(intent.dir);
        break;
      case "Discard":
        clearTimer();
        setDraft(initial);
        onEndEdit();
        break;
      case "None":
        break;
    }
  };

  if (!editing || locked) {
    const display = field === "title" ? (item.title || "Untitled") : (item.note?.body ?? "");
    if (field === "note" && !item.note) return null;
    const dim = field === "title" && (item.status === "completed" || item.status === "in-progress");
    return (
      <Text
        size={field === "title" ? "2" : "1"}
        color={field === "title" ? (dim ? "gray" : undefined) : "gray"}
        onClick={() => {
          if (!locked) onBeginEdit();
        }}
        style={{ cursor: locked ? "default" : "text" }}
      >
        {display}
      </Text>
    );
  }

  return (
    <TextArea
      ref={ref}
      size={field === "title" ? "2" : "1"}
      autoFocus
      value={draft}
      rows={1}
      onChange={(e) => {
        setDraft(e.target.value);
        scheduleAutosave(e.target.value);
      }}
      onCompositionStart={() => {
        composingRef.current = true;
        clearTimer();
      }}
      onCompositionEnd={(e) => {
        composingRef.current = false;
        scheduleAutosave((e.target as HTMLTextAreaElement).value);
      }}
      onKeyDown={(e) => {
        const intent = reduceKey({
          key: e.key,
          metaKey: e.metaKey,
          shiftKey: e.shiftKey,
          caretAtStart: caretAtStart(),
          caretAtEnd: caretAtEnd(),
          value: draft,
          field,
          composing: composingRef.current || (e as unknown as { isComposing?: boolean }).isComposing === true,
        });
        if (intent.type !== "None") {
          e.preventDefault();
          void execute(intent);
        }
      }}
      onBlur={() => {
        clearTimer();
        if (!composingRef.current) void save(draft); // blur commits (Swift focus-loss save)
      }}
    />
  );
}
```

Critical wiring recap: local `draft` state (snapshot never clobbers typing); `composingRef` gates both the debounce and the Enter/split intents (IME safety); `ifCurrent: item` on every edit; `Split` branches on caret-at-end (create below) vs mid-caret (`split_item`); `MergeIntoPrevious` is gated by `canMergeInto` (Swift precondition: previous draft/ready + no note); `DeleteEmpty` deletes the item (title) or the note. `update_note` uses the plain (non-if-current) wrapper per the core divergence.

- [ ] **Step 2: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green (no DOM tests for this component by policy; the reducer it calls is covered in Task 9).

- [ ] **Step 3: Commit**

```bash
git add src/components/InlineEditor.tsx
git commit -m "feat(ui): inline title/note editor with IME-safe autosave

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 12: `TaskRow.tsx` — inline editor host, status leading-click, item menu

Integrate `InlineEditor` for title + note; status leading-click (left → `leadingStatusClickTarget` via `setStatus`, right → `rightClickStatusTarget` = draft); item `DropdownMenu`/`ContextMenu` (status submenu of all statuses, Move to Collection submenu, Delete). Parity: `TaskRow.swift` status click; master spec §6 item menu.

> **Radix 3.3.0 latitude:** `ContextMenu`/`DropdownMenu` parts (`Root/Trigger/Content/Item/Sub/SubTrigger/SubContent/Separator`). If 3.3.0 names differ, adjust; gate = clean tsc+build.

**Files:**
- Modify: `src/components/TaskRow.tsx`

- [ ] **Step 1: Rewrite `TaskRow.tsx`**

```tsx
import { ContextMenu, Flex, Text } from "@radix-ui/themes";
import { DotFilledIcon } from "@radix-ui/react-icons";
import type { CollectionColor, CollectionSummary, Snapshot, TaskItem, TaskStatus } from "../api/types";
import { setStatus, moveItem, deleteItem } from "../api/client";
import { leadingStatusClickTarget, rightClickStatusTarget } from "../state/status";
import type { FocusDir } from "../state/editor";
import { InlineEditor } from "./InlineEditor";

const STATUS_COLOR: Record<TaskStatus, CollectionColor> = {
  draft: "gray", ready: "gray", "in-progress": "blue", completed: "green",
  "on-hold": "orange", rejected: "red", aborted: "red",
};

const ALL_STATUSES: TaskStatus[] = [
  "draft", "ready", "in-progress", "completed", "on-hold", "rejected", "aborted",
];

export interface TaskRowProps {
  item: TaskItem;
  previous?: TaskItem;
  showCollection: boolean;
  collections: CollectionSummary[];
  focused: boolean;
  editingField: "title" | "note" | null;
  onFocus: () => void;
  onEditTitle: () => void;
  onEditNote: () => void;
  onEndEdit: () => void;
  onMoveFocus: (dir: FocusDir) => void;
  onSnapshot: (snap: Snapshot) => void;
}

export function TaskRow({
  item, previous, showCollection, collections,
  focused, editingField, onFocus, onEditTitle, onEditNote, onEndEdit, onMoveFocus, onSnapshot,
}: TaskRowProps) {
  const advance = (e: React.MouseEvent) => {
    e.stopPropagation();
    setStatus(leadingStatusClickTarget(item.status), item.id, item).then(onSnapshot).catch(console.error);
  };
  const toDraft = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setStatus(rightClickStatusTarget(item.status), item.id, item).then(onSnapshot).catch(console.error);
  };

  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger>
        <Flex
          align="start"
          gap="2"
          py="1"
          onClick={onFocus}
          style={{ background: focused ? "var(--accent-3)" : undefined, borderRadius: 4 }}
        >
          <Text
            color={STATUS_COLOR[item.status]}
            onClick={advance}
            onContextMenu={toDraft}
            style={{ cursor: "pointer" }}
            title={item.status}
          >
            <DotFilledIcon />
          </Text>
          <Flex direction="column" flexGrow="1">
            <InlineEditor
              item={item}
              field="title"
              previous={previous}
              editing={editingField === "title"}
              onBeginEdit={onEditTitle}
              onEndEdit={onEndEdit}
              onMoveFocus={onMoveFocus}
              onSnapshot={onSnapshot}
            />
            <InlineEditor
              item={item}
              field="note"
              editing={editingField === "note"}
              onBeginEdit={onEditNote}
              onEndEdit={onEndEdit}
              onMoveFocus={onMoveFocus}
              onSnapshot={onSnapshot}
            />
          </Flex>
          {showCollection ? (
            <Text size="1" color="gray">{item.collection}</Text>
          ) : null}
        </Flex>
      </ContextMenu.Trigger>

      <ContextMenu.Content>
        <ContextMenu.Sub>
          <ContextMenu.SubTrigger>Status</ContextMenu.SubTrigger>
          <ContextMenu.SubContent>
            {ALL_STATUSES.map((s) => (
              <ContextMenu.Item
                key={s}
                onSelect={() =>
                  setStatus(s, item.id, item).then(onSnapshot).catch(console.error)
                }
              >
                {s}
              </ContextMenu.Item>
            ))}
          </ContextMenu.SubContent>
        </ContextMenu.Sub>

        <ContextMenu.Sub>
          <ContextMenu.SubTrigger>Move to Collection</ContextMenu.SubTrigger>
          <ContextMenu.SubContent>
            {collections.map((c) => (
              <ContextMenu.Item
                key={c.name}
                disabled={c.name === item.collection}
                onSelect={() => moveItem(item.id, c.name).then(onSnapshot).catch(console.error)}
              >
                {c.displayName}
              </ContextMenu.Item>
            ))}
          </ContextMenu.SubContent>
        </ContextMenu.Sub>

        <ContextMenu.Separator />
        <ContextMenu.Item
          color="red"
          onSelect={() => deleteItem(item.id).then(onSnapshot).catch(console.error)}
        >
          Delete
        </ContextMenu.Item>
      </ContextMenu.Content>
    </ContextMenu.Root>
  );
}
```

Critical wiring: left-click status dot → `setStatus(leadingStatusClickTarget(...), id, item)`; `onContextMenu` on the dot → draft (calls `e.preventDefault()` so the row context menu does not also open — if 3.3.0's `ContextMenu.Trigger` still intercepts, gate the dot's right-click via a `stopPropagation` and accept the menu, adjusting to the installed API). Title/note each render an `InlineEditor`. The item context menu drives status/move/delete, each replacing the snapshot from the result.

- [ ] **Step 2: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
(DetailPane passes the new props in Task 13; if gating in isolation fails on missing props from DetailPane, implement Task 13 then re-gate.)

- [ ] **Step 3: Commit**

```bash
git add src/components/TaskRow.tsx
git commit -m "feat(ui): task row inline editing, status click, item menu

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 13: `Sidebar.tsx` + `DetailPane.tsx` — collection/group menus, footer toggles, new-task affordance

Collection menu (Rename, Color submenu, Archive/Unarchive, Move to Group, Clear All/Completed, Delete w/ AlertDialog), group menu (Rename, Delete w/ AlertDialog, Add Collection), footer `DropdownMenu` (Hide Completed, Show Archived checkboxes), DetailPane "new task" affordance (→ `createItem`). Parity: `SidebarView.swift` (`CollectionActionMenuItems`, `archivedCollectionActionMenuItems`, group context menu, footer `Toggle`s); master spec §6.

> **Radix 3.3.0 latitude:** `DropdownMenu`/`ContextMenu` parts incl. `DropdownMenu.CheckboxItem`. For inline rename, this plan uses `window.prompt` (stock, no extra deps) to obtain the new name, matching the spec's "minimal" stance; if a Radix dialog-based rename is preferred later that is a Phase-5 polish. Adjust part/prop names to the installed 3.3.0 API; gate = clean tsc+build.

**Files:**
- Modify: `src/components/Sidebar.tsx`, `src/components/DetailPane.tsx`

- [ ] **Step 1: Rewrite `Sidebar.tsx`**

```tsx
import { Badge, Box, Button, ContextMenu, DropdownMenu, Flex, Text } from "@radix-ui/themes";
import { DotFilledIcon, GearIcon } from "@radix-ui/react-icons";
import type { CollectionColor, CollectionSummary, Snapshot } from "../api/types";
import {
  ALL_COLLECTION, allIncompleteCount, sidebarGroups,
} from "../state/view";
import type { ConfirmRequest } from "../state/confirm";
import {
  clearItems, createCollection, createGroup, deleteCollection, deleteGroup,
  moveCollection, renameCollection, renameGroup, setCollectionArchived, setCollectionColor,
} from "../api/client";

const COLORS: CollectionColor[] = ["gray", "red", "orange", "yellow", "green", "blue", "purple"];

export interface SidebarProps {
  snapshot: Snapshot;
  selected: string;
  showArchived: boolean;
  hideCompleted: boolean;
  onSelect: (name: string) => void;
  onToggleHideCompleted: () => void;
  onToggleShowArchived: () => void;
  onSnapshot: (snap: Snapshot) => void;
  onRequestConfirm: (req: ConfirmRequest) => void;
}

export function Sidebar({
  snapshot, selected, showArchived, hideCompleted,
  onSelect, onToggleHideCompleted, onToggleShowArchived, onSnapshot, onRequestConfirm,
}: SidebarProps) {
  const groupNames = snapshot.groups.map((g) => g.name);

  const renameCol = (c: CollectionSummary) => {
    const next = window.prompt("Rename collection", c.displayName);
    if (next && next.trim()) {
      renameCollection(c.name, next.trim()).then(onSnapshot).catch(console.error);
    }
  };
  const renameGrp = (name: string) => {
    const next = window.prompt("Rename group", name);
    if (next && next.trim()) renameGroup(name, next.trim()).then(onSnapshot).catch(console.error);
  };
  const addCollectionTo = (group: string) => {
    const name = window.prompt("New collection name");
    if (name && name.trim()) createCollection(name.trim(), group).then(onSnapshot).catch(console.error);
  };

  return (
    <Flex direction="column" gap="1" p="2" style={{ width: 240 }}>
      <Button variant={selected === ALL_COLLECTION ? "soft" : "ghost"} onClick={() => onSelect(ALL_COLLECTION)}>
        <Flex align="center" gap="2" flexGrow="1">
          <Box flexGrow="1"><Text align="left">All</Text></Box>
          <Badge>{allIncompleteCount(snapshot)}</Badge>
        </Flex>
      </Button>

      {sidebarGroups(snapshot, showArchived).map((group) => (
        <Box key={group.name} mt="2">
          <ContextMenu.Root>
            <ContextMenu.Trigger>
              <Text size="1" color="gray">{group.name === "DefaultGroup" ? "No Group" : group.name}</Text>
            </ContextMenu.Trigger>
            <ContextMenu.Content>
              <ContextMenu.Item disabled={group.name === "DefaultGroup"} onSelect={() => renameGrp(group.name)}>
                Rename Group
              </ContextMenu.Item>
              <ContextMenu.Item onSelect={() => addCollectionTo(group.name)}>Add Collection</ContextMenu.Item>
              <ContextMenu.Separator />
              <ContextMenu.Item
                color="red"
                disabled={group.name === "DefaultGroup"}
                onSelect={() =>
                  onRequestConfirm({
                    title: `Delete group "${group.name}"?`,
                    description: "Its collections move to No Group. This cannot be undone.",
                    confirmLabel: "Delete",
                    onConfirm: () => deleteGroup(group.name).then(onSnapshot).catch(console.error),
                  })
                }
              >
                Delete Group
              </ContextMenu.Item>
            </ContextMenu.Content>
          </ContextMenu.Root>

          {group.collections.map((c) => (
            <ContextMenu.Root key={c.name}>
              <ContextMenu.Trigger>
                <Button variant={selected === c.name ? "soft" : "ghost"} onClick={() => onSelect(c.name)}>
                  <Flex align="center" gap="2" flexGrow="1">
                    <Text color={c.color}><DotFilledIcon /></Text>
                    <Box flexGrow="1"><Text align="left">{c.displayName}</Text></Box>
                    <Badge>{c.incompleteCount}</Badge>
                  </Flex>
                </Button>
              </ContextMenu.Trigger>
              <ContextMenu.Content>
                <ContextMenu.Item onSelect={() => renameCol(c)}>Rename</ContextMenu.Item>

                <ContextMenu.Sub>
                  <ContextMenu.SubTrigger>Color</ContextMenu.SubTrigger>
                  <ContextMenu.SubContent>
                    {COLORS.map((color) => (
                      <ContextMenu.Item
                        key={color}
                        onSelect={() => setCollectionColor(c.name, color).then(onSnapshot).catch(console.error)}
                      >
                        <Text color={color}><DotFilledIcon /></Text> {color}
                      </ContextMenu.Item>
                    ))}
                  </ContextMenu.SubContent>
                </ContextMenu.Sub>

                <ContextMenu.Item
                  onSelect={() =>
                    setCollectionArchived(c.name, !c.isArchived).then(onSnapshot).catch(console.error)
                  }
                >
                  {c.isArchived ? "Unarchive" : "Archive"}
                </ContextMenu.Item>

                <ContextMenu.Sub>
                  <ContextMenu.SubTrigger>Move to Group</ContextMenu.SubTrigger>
                  <ContextMenu.SubContent>
                    {groupNames.map((g) => (
                      <ContextMenu.Item
                        key={g}
                        disabled={g === c.groupName}
                        onSelect={() => moveCollection(c.name, g).then(onSnapshot).catch(console.error)}
                      >
                        {g === "DefaultGroup" ? "No Group" : g}
                      </ContextMenu.Item>
                    ))}
                  </ContextMenu.SubContent>
                </ContextMenu.Sub>

                <ContextMenu.Sub>
                  <ContextMenu.SubTrigger>Clear</ContextMenu.SubTrigger>
                  <ContextMenu.SubContent>
                    <ContextMenu.Item onSelect={() => clearItems(c.name, false).then(onSnapshot).catch(console.error)}>
                      All Items
                    </ContextMenu.Item>
                    <ContextMenu.Item onSelect={() => clearItems(c.name, true).then(onSnapshot).catch(console.error)}>
                      Completed Items
                    </ContextMenu.Item>
                  </ContextMenu.SubContent>
                </ContextMenu.Sub>

                <ContextMenu.Separator />
                <ContextMenu.Item
                  color="red"
                  onSelect={() =>
                    onRequestConfirm({
                      title: `Delete collection "${c.displayName}"?`,
                      description: "All its tasks are permanently deleted. This cannot be undone.",
                      confirmLabel: "Delete",
                      onConfirm: () => deleteCollection(c.name).then(onSnapshot).catch(console.error),
                    })
                  }
                >
                  Delete
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Root>
          ))}
        </Box>
      ))}

      <Box flexGrow="1" />

      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          <Button variant="ghost"><GearIcon /> View</Button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content>
          <DropdownMenu.CheckboxItem checked={hideCompleted} onCheckedChange={onToggleHideCompleted}>
            Hide Completed
          </DropdownMenu.CheckboxItem>
          <DropdownMenu.CheckboxItem checked={showArchived} onCheckedChange={onToggleShowArchived}>
            Show Archived
          </DropdownMenu.CheckboxItem>
          <DropdownMenu.Separator />
          <DropdownMenu.Item
            onSelect={() => {
              const name = window.prompt("New group name");
              if (name && name.trim()) createGroup(name.trim()).then(onSnapshot).catch(console.error);
            }}
          >
            Add a Group
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </Flex>
  );
}
```

Maps to Swift `SidebarView`: collection menu = Rename / Color / Archive-Unarchive / Move-to-Group / Clear(All|Completed) / Delete; group menu = Rename / Add Collection / Delete; footer toggles = Hide Completed + Show Archived (plus Add a Group, mirroring the Swift footer `Menu`). Destructive collection/group deletes route through `onRequestConfirm` (the App AlertDialog).

- [ ] **Step 2: Rewrite `DetailPane.tsx`**

```tsx
import { Button, Flex, Heading, ScrollArea, TextField } from "@radix-ui/themes";
import { MagnifyingGlassIcon, PlusIcon } from "@radix-ui/react-icons";
import type { Snapshot } from "../api/types";
import { ALL_COLLECTION, visibleItems, type ViewState } from "../state/view";
import { createItem } from "../api/client";
import type { ConfirmRequest } from "../state/confirm";
import type { FocusDir } from "../state/editor";
import { TaskRow } from "./TaskRow";

export interface DetailPaneProps {
  snapshot: Snapshot;
  view: ViewState;
  focusedId: string | null;
  editingId: string | null;
  onSearch: (q: string) => void;
  onFocusItem: (id: string | null) => void;
  onEditItem: (id: string | null) => void;
  onSnapshot: (snap: Snapshot) => void;
  onRequestConfirm: (req: ConfirmRequest) => void;
}

export function DetailPane({
  snapshot, view, focusedId, editingId,
  onSearch, onFocusItem, onEditItem, onSnapshot,
}: DetailPaneProps) {
  const items = visibleItems(snapshot, view);
  const title = view.selected === ALL_COLLECTION
    ? "All"
    : snapshot.collections.find((c) => c.name === view.selected)?.displayName ?? view.selected;

  const newTask = () => {
    const target = view.selected === ALL_COLLECTION ? undefined : view.selected;
    createItem(target)
      .then((snap) => {
        onSnapshot(snap);
        const created = [...snap.items].reverse()
          .find((i) => i.title === "" && i.status === "draft" && (!target || i.collection === target));
        if (created) {
          onFocusItem(created.id);
          onEditItem(created.id);
        }
      })
      .catch(console.error);
  };

  const moveFocus = (dir: FocusDir) => {
    if (items.length === 0) return;
    const idx = items.findIndex((i) => i.id === focusedId);
    const nextIdx = dir === "down"
      ? Math.min(items.length - 1, (idx < 0 ? -1 : idx) + 1)
      : Math.max(0, (idx < 0 ? items.length : idx) - 1);
    const next = items[nextIdx];
    if (next) {
      onFocusItem(next.id);
      onEditItem(next.id); // entering a row opens its title editor (matches Swift focus model)
    }
  };

  return (
    <Flex direction="column" flexGrow="1" p="3" gap="3">
      <Flex align="center" justify="between">
        <Heading size="4">{title}</Heading>
        <Flex align="center" gap="2">
          <TextField.Root placeholder="Search" value={view.search} onChange={(e) => onSearch(e.target.value)}>
            <TextField.Slot><MagnifyingGlassIcon /></TextField.Slot>
          </TextField.Root>
          <Button onClick={newTask}><PlusIcon /> New Task</Button>
        </Flex>
      </Flex>
      <ScrollArea>
        <Flex direction="column">
          {items.map((item, i) => (
            <TaskRow
              key={item.id}
              item={item}
              previous={i > 0 ? items[i - 1] : undefined}
              showCollection={view.selected === ALL_COLLECTION}
              collections={snapshot.collections}
              focused={focusedId === item.id}
              editingField={editingId === item.id ? "title" : null}
              onFocus={() => onFocusItem(item.id)}
              onEditTitle={() => onEditItem(item.id)}
              onEditNote={() => onEditItem(item.id)}
              onEndEdit={() => onEditItem(null)}
              onMoveFocus={moveFocus}
              onSnapshot={onSnapshot}
            />
          ))}
        </Flex>
      </ScrollArea>
    </Flex>
  );
}
```

Note: `editingField` here is simplified to title-only when a row is the `editingId`. If you want independent title/note edit targets, lift an `{ id, field }` editing descriptor into `App.tsx` and thread it; the spec's bar is reached with title-on-enter and click-to-edit-note (note click calls `onEditNote`, which the implementer can wire to a richer editing descriptor — adjust within the established latitude, gate = tsc+build). The `previous` prop feeds the Backspace-merge precondition in `InlineEditor`.

- [ ] **Step 3: Full frontend gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green. Resolve any 3.3.0 API drift (menu part names, `CheckboxItem` prop) to the installed API and note the change.

- [ ] **Step 4: Commit**

```bash
git add src/components/Sidebar.tsx src/components/DetailPane.tsx
git commit -m "feat(ui): collection/group menus, footer toggles, new-task affordance

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 14: Final gate + docs

**Files:**
- Modify: `README.md` (only if the run instructions changed)

- [ ] **Step 1: Full workspace + frontend gate**

```bash
cargo test
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
npx vitest run
npx tsc --noEmit
npm run build
```
Expected: all green/clean. (`cargo test` = pond-core + taskpond-cli + pond-tauri.)

- [ ] **Step 2: Manual launch (human visual check — not gated by CI)**

```bash
cargo tauri dev
```
Spot-check: Cmd+N creates a focused empty draft; type a title; Enter splits at caret; Backspace at start of a row whose previous row is a note-free draft/ready merges; click a status dot to advance; right-click a dot sets draft; collection/group menus; footer Hide Completed / Show Archived; delete a collection → AlertDialog. (If `cargo tauri` is unavailable in the environment, skip — the build gate already proves compilation.)

- [ ] **Step 3: Update README run notes only if needed**

If the existing "how to run the Tauri desktop app" doc (commit `94e067d`) is still accurate, make no change. Otherwise update the run/shortcut notes to mention Cmd+N / Cmd+Backspace and the editor.

- [ ] **Step 4: Commit (only if files changed)**

```bash
git add -A
git commit -m "docs: Phase 4 run notes + final gate

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

If nothing changed in Step 3, skip the commit — Phase 4 is complete after the gate passes.
