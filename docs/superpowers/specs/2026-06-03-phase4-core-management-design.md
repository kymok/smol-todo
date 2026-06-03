# Phase 4: Core Management (GUI mutations + inline editor) — Design

- **Date:** 2026-06-03
- **Status:** Approved design
- **Phase:** 4 of 5 (see `2026-06-02-tauri-radix-migration-design.md`)
- **Builds on:** Phase 1 `pond-core`, Phase 2 `taskpond-cli`, Phase 3 read-only Tauri shell (all complete).

## 1. Overview

Turn the Phase 3 read-only shell into a day-to-day-usable editor. Phase 4 adds the
mutation IPC layer (thin wrappers over `pond-core`) and the frontend UI that drives it:
status changes, create/delete, notes, the **full inline title/note editor (including
merge/split)**, and collection/group management. `pond-core` already exposes every
operation required — **no Rust core changes** — so this phase is entirely `pond-tauri`
(commands) + frontend (state, components, menus, the editor).

### Goals
- Every core mutation is reachable from the GUI and the UI re-renders from Rust truth.
- The inline editor reproduces the Swift behavior: click-to-edit, 500 ms debounced
  autosave (IME-safe), `Enter` split-at-caret, `Backspace`-at-start merge, `Cmd+Enter`
  confirm, `Tab`→note/next, `Esc` discard, `↑/↓` row navigation, empty-title-deletes.
- Collection/group CRUD + color + archive + show-archived; counts and the "All" view
  (already present) keep working through mutations.
- The app is "day-to-day usable" (the master-spec Phase 4 bar).

### Non-goals (Phase 4 — deferred to Phase 5)
- **Auto-draft on edit** (`usesAutoDraft`): in Phase 4, editing does **not** change status.
- Bulk-status dialog (`set_statuses`), prompt editing + Copy Prompt / Copy CLI Command,
  export save-dialog, Settings (incl. persisted settings + CLI-install UI), always-on-top,
  file-drop-to-create, Copy-ID / clipboard, and rich error dialogs.
- **Reorder** of items/collections/groups (permanently out of scope per the user).

## 2. Confirmed decisions

1. **Mutations return a fresh snapshot.** Each mutation command performs the op and returns
   the rebuilt `SnapshotDto`; the frontend replaces its state from the result (master spec
   §5). The existing `store-changed` watcher remains **only** for *external* edits (the
   `taskpond` CLI). This avoids optimistic-update divergence and the watcher's
   write→fs-event→debounce→refetch latency for the user's own actions.
2. **Single managed `TaskStore`.** Hold one `TaskStore` in Tauri state via `app.manage`;
   `get_snapshot` and all mutation commands use it instead of `open_default()` per call.
   `TaskStore` is path-only and takes the advisory file lock per operation, so concurrent
   command invocations stay safe and remain serialized against the CLI.
3. **Full inline editor in Phase 4** (user decision), including `merge_item`/`split_item`.
   Phase 5 keeps only auto-draft / bulk / prompts / export / settings / OS-integration.
4. **Optimistic concurrency via `*_if_current`.** Editing actions (title/collection/status/
   note autosave, leading status-click) pass the known `TaskItem` as `ifCurrent`, mirroring
   the Swift `AppModel` (`setStatus(...ifCurrent:)`, `addNote(...ifCurrent:)`, etc.); a stale
   write is skipped rather than clobbering a concurrent change. Non-contended ops
   (create, collection/group CRUD) use the plain variants.
5. **Radix defaults only.** Stock Radix Themes parts; built-in named palette for status/
   collection colors; no theme customization. Visual polish is the user's later pass.
6. **Minimal confirm for destructive deletes.** Deleting a collection or group shows a stock
   Radix `AlertDialog`; item delete (and empty-title-delete) is immediate. Elaborate *error*
   dialogs are Phase 5.
7. **View toggles are session-local** in Phase 4 (Hide Completed, Show Archived) — React
   state extending `view.ts`. Persistence (`get_settings`/`set_setting`) is Phase 5.
8. **Verification = logic + build gates + manual launch; no visual/DOM tests** (user
   decision, carried from Phase 3). The editor's key handling is factored as a pure,
   unit-tested reducer.

## 3. Architecture

```
src-tauri/src/
├─ main.rs        register mutation commands; app.manage(TaskStore::open_default())
├─ state.rs       (new) managed-store wiring / State<TaskStore> helper + snapshot rebuild
├─ commands.rs    get_snapshot + all mutation commands (thin wrappers → pond-core), each -> Result<SnapshotDto, String>
├─ dto.rs         (unchanged) SnapshotDto / CollectionSummaryDto / CollectionGroupSummaryDto
└─ watcher.rs     (unchanged) store-changed for external edits

src/
├─ api/
│  ├─ client.ts   add typed wrappers for every mutation command (invoke → Snapshot)
│  └─ types.ts    (unchanged DTO mirrors)
├─ state/
│  ├─ view.ts     extend: hide-completed, show-archived, selection helpers
│  └─ editor.ts   (new) pure key-handling reducer: (key event + caret/field state) → Intent
├─ components/
│  ├─ Sidebar.tsx       + collection/group context menus, footer toggles, create/rename/delete/color/archive
│  ├─ DetailPane.tsx    + "new task" affordance, wires mutations
│  ├─ TaskRow.tsx       + status leading-click, status/move/delete menu, note line
│  └─ InlineEditor.tsx  (new) Text⇄TextArea swap, local draft, debounced autosave, key map
└─ App.tsx        snapshot replace-from-mutation-result; Cmd+N / delete shortcuts; AlertDialog host
```

Each unit stays focused: `api/` is the only place that calls `invoke`; `state/` holds the
pure logic (selectors + the editor reducer) that the tests exercise; components render and
dispatch. `commands.rs` wrappers contain no logic beyond mapping args → `pond-core` call →
rebuilt snapshot → string error.

## 4. IPC command surface (Phase 4)

All commands take `State<TaskStore>` and return `Result<SnapshotDto, String>` (the rebuilt
snapshot; `Err` is the `StoreError` parity message). `get_snapshot` is migrated to the
managed store. Mapping to `pond-core`:

| Command | `pond-core` call |
|---|---|
| `create_item(collection?)` — new empty **draft**, title typed in the editor (Cmd+N / header affordance) | `add("", collection|DEFAULT_COLLECTION, None, allow_empty_title=true, Draft)` |
| `update_item(id, title?, collection?, status?, ifCurrent?)` | `update` / `update_if_current` |
| `set_status(status, id, ifCurrent?)` | `set_status` / `set_status_if_current` |
| `move_item(id, collection)` | `move_item` |
| `delete_item(id)` / `delete_items(ids, collection?)` | `delete` / `delete_many` |
| `add_note(id, body, ifCurrent?)` / `update_note(id, body, ifCurrent?)` / `delete_note(id, ifCurrent?)` | `add_note` / `update_note` / `delete_note` (+ `_if_current`) |
| `merge_item(...)` / `split_item(...)` | `merge_item` / `split_item` |
| `create_collection(name, group?)` | `create_collection(name, group|DEFAULT_GROUP)` |
| `rename_collection(old, new)` | `rename_collection` |
| `set_collection_color(name, color)` | `set_collection_color` |
| `set_collection_archived(name, archived)` | `set_collection_archived` |
| `move_collection(name, group)` | `move_collection` |
| `clear_items(name, completed_only)` | `clear_items` |
| `delete_collection(name)` | `delete_collection` |
| `create_group(name)` / `rename_group(old,new)` / `delete_group(name)` | `create_group` / `rename_group` / `delete_group` |

- **`SnapshotDto` is unchanged** from Phase 3 (`{ items, collections, groups }`).
- **Capabilities unchanged:** custom app commands need no capability entry in Tauri v2
  (confirmed in the Phase 3 review); no new plugins this phase.
- `status`/`color` cross the wire as their serde rawValue strings (the TS unions already
  mirror them); the command parses them to the `pond-core` enums.
- **Create = Draft.** A newly created item is a `Draft` with an empty title, opened for
  editing. Without auto-draft (Phase 5), it stays `Draft` until the user changes its status
  (status-click/menu) — this matches the master-spec phasing (Cmd+N "new draft" in Phase 4,
  auto-draft promotion in Phase 5).

## 5. Frontend state & the inline editor

### State
- The mutation client wrappers return `Snapshot`; `App` replaces snapshot state from each
  result. `store-changed` still triggers a re-fetch for external (CLI) edits.
- View state (selected collection, search, **hideCompleted**, **showArchived**) is local
  React state; `view.ts` gains `hideCompleted` filtering (hide `completed`) and
  `showArchived` (already a parameter of `sidebarGroups`).

### Inline editor (highest-risk unit — `InlineEditor.tsx` + `state/editor.ts`)
- **Mount model:** a display `Text` that swaps to a Radix `TextArea` on click, placing the
  caret at the click (mirrors Swift, which overlays the editor only while editing). On blur/
  commit it swaps back. In-progress / completed tasks **lock title & collection editing**
  (the note stays editable) — per `TaskRow.swift` / master spec §7.
- **Local draft:** the editing field holds local state independent of the snapshot, so an
  incoming snapshot (autosave result or a CLI edit) never clobbers in-progress typing.
- **Autosave:** 500 ms debounced (matches Swift `Task.sleep(500_000_000)`), **skipped during
  IME composition** (`compositionstart`/`compositionend`) — essential for Japanese input —
  via the `*_if_current` commands so a stale field can't overwrite a concurrent change.
- **Key map** (title field), reproduced from `TaskRow.swift`:
  - `Enter` → **split at caret** into two tasks (`split_item`); on an empty/caret-at-end case,
    creates a task below.
  - `Backspace` at caret position 0 → **merge into the previous task** (`merge_item`); the
    frontend gates this on the Swift precondition (previous task is draft/ready with no note),
    mirroring `mergeWithPrevious` in `AppModel.swift`. Exact gate verified against the Swift
    source in the plan.
  - `Cmd+Enter` → confirm and move to note/next; `Tab` → save then note/next; `Esc` → discard
    local draft; `↑`/`↓` → move focus between rows; empty title on commit → `delete_item`.
- **Note field:** click-to-edit, debounced autosave, `Return`/`Tab` → next, `Esc` → discard,
  empty note on save → `delete_note`.
- **Testability:** `state/editor.ts` is a **pure reducer** mapping
  `(keyEvent, caret, fieldState)` → an `Intent` (`Split | Merge | Commit | MoveFocus(dir) |
  DeleteEmpty | Discard | None`). The component executes intents (calls the client); the
  reducer is unit-tested with no DOM. Exact bindings/edge cases are pinned in the plan from
  the Swift source.

## 6. Menus & interactions (stock Radix)

- **Status leading-click** (`TaskRow`): left-click the status icon advances per
  `leadingStatusClickTarget` — **ready → completed, in-progress → completed, otherwise →
  ready**; right-click sets **draft**. Both via `set_status` with `ifCurrent`.
- **Item menu** (`ContextMenu`/`DropdownMenu`): status submenu (all statuses), Move to
  Collection (submenu of collections), Delete.
- **Collection menu:** Rename (inline rename), Color (named-palette submenu), Archive /
  Unarchive, Move to Group (submenu), Clear (All / Completed), Delete.
- **Group menu:** Rename, Delete, Add Collection.
- **Sidebar footer** (`DropdownMenu` with checkbox items): Hide Completed, Show Archived.
- **Create:** `Cmd+N` creates a new draft in the selected collection (the "All" view falls
  back to the default collection) and focuses its title editor; a visible "new task"
  affordance in the detail header does the same.
- **Delete shortcut:** `Cmd+⌫` deletes the focused task.
- **Destructive confirm:** deleting a collection/group opens a Radix `AlertDialog`.

## 7. Behavior parity notes (source of truth = Swift)

The implementation plan encodes exact values; key behaviors (from `TaskRow.swift`,
`TaskViewSupport.swift`, `AppModel.swift`, master spec §7):

- Status leading-click target table (§6); right-click → draft.
- In-progress / completed lock inline title & collection editing; note still editable.
- 500 ms debounced autosave; skipped during IME composition; `*_if_current` everywhere an
  edit targets an existing item.
- `Enter` split-at-caret; `Backspace`-at-start merge (valid-merge only); `Cmd+Enter` confirm;
  `Tab` save→note/next; `Esc` discard; `↑/↓` row focus; empty title commit → delete.
- Note: autosave; `Return`/`Tab` → next; `Esc` discard; empty note on save → delete note.
- **No auto-draft** in Phase 4 (deferred): edits do not change status.

## 8. Testing / verification

- **Rust (`pond-tauri`):** per-command tests against a `tempdir` store asserting the
  post-mutation snapshot reflects the op (e.g., `create_item` adds an item;
  `set_collection_archived` flips `isArchived`; `delete_collection` removes it). Reuse the
  Phase 3 harness (`build_snapshot` is already test-covered).
- **Frontend logic (Vitest):** the `state/editor.ts` reducer (split/merge/commit/focus/
  delete-empty/discard intents + IME guard), and extended `view.ts` selectors
  (hideCompleted, showArchived).
- **Gates:** the full Phase 3 gate — `cargo test && cargo clippy --workspace --all-targets
  -- -D warnings && cargo fmt --all -- --check` and `npx vitest run && npx tsc --noEmit &&
  npm run build` — stay green.
- **No visual/DOM/screenshot tests** (user decision). Manual launch (`cargo tauri dev`) is
  the human visual check.

## 9. References (behavioral source of truth)
- `Sources/PondApp/TaskRow.swift` — inline editor, key handling, status click, menus.
- `Sources/PondApp/TaskViewSupport.swift` — `leadingStatusClickTarget`, status icons.
- `Sources/PondApp/AppModel.swift` — mutation orchestration, `ifCurrent` usage, shortcuts.
- `Sources/PondApp/SidebarView.swift` — collection/group menus, footer toggles.
- `crates/pond-core/src/store.rs` — the mutation API (incl. `*_if_current`, `merge_item`,
  `split_item`, `set_collection_archived`).
- Master migration spec — `2026-06-02-tauri-radix-migration-design.md` (§5 IPC, §6 GUI map,
  §7 parity, §9 phasing).
