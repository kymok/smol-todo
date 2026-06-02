# Phase 3: Tauri Shell + IPC + Radix Themes Scaffold — Design

- **Date:** 2026-06-02
- **Status:** Approved design
- **Phase:** 3 of 5 (see `2026-06-02-tauri-radix-migration-design.md`)
- **Builds on:** Phase 1 `pond-core` (data store) and Phase 2 `taskpond-cli` (both complete).

## 1. Overview

Stand up the desktop app shell: a Tauri v2 application (`pond-tauri` crate) hosting a
Vite + React + TypeScript + Radix Themes frontend that **renders the store read-only**.
This phase proves the stack end-to-end — real data from `pond-core` via one IPC command,
a file-watcher for live updates, and the two-pane sidebar/detail layout — without any
mutations. Editing, menus, settings, and OS integration come in Phases 4–5.

### Goals
- The app launches, fetches a snapshot from `pond-core`, and renders the sidebar
  (collections/groups) + task list, all built from stock Radix Themes parts.
- Selecting a collection, searching, and an incomplete-only filter work (client-side,
  read-only).
- A `taskpond` CLI edit to the store appears live (file-watcher → re-fetch).

### Non-goals (Phase 3)
- Any mutation (create/edit/delete/status/notes/collection ops) — Phases 4–5.
- Context menus, dropdown menus, settings, always-on-top, file-drop, drag-reorder.
- **Automated visual-regression / screenshot tests** — the design will be tweaked, so
  committed visual baselines would be churn. Verification is logic + build + manual launch
  (see §7).

## 2. Confirmed decisions

1. **Toolchain — pin via `rust-toolchain.toml`.** Tauri v2 needs Rust ≥ 1.77.2; the env has
   1.72.1. Add `rust-toolchain.toml` (`channel = "1.96.0"`, profile minimal) at the repo
   root; rustup auto-installs it for this repo only (global default untouched). `1.96.0` is
   the current stable (confirmed reachable via `rustup check`). The existing 1.72-era
   dep pins (`getrandom`/`tempfile`/`clap`) still build fine on 1.96 and are **left as-is**
   to avoid behavior churn; relaxing them is an optional later cleanup, out of scope here.
2. **Scaffolding — manual integration into the existing Cargo workspace** (not
   `create-tauri-app`, which makes a standalone repo). Add a `pond-tauri` crate under
   `src-tauri/` as a workspace member; add the Vite/React/TS frontend at the repo root.
   **Package manager: npm.**
3. **IPC wire types — `pond-tauri` owns its DTOs.** `pond-core`'s summary types are not
   `Serialize` (deliberate — the CLI defined its own output structs). `pond-tauri` defines
   serializable, camelCase snapshot DTOs with the fuller field set the GUI needs.
   `TaskItem` already serializes (camelCase dates, singular `note`, kebab status), so items
   are sent as `TaskItem` JSON directly; only the summaries get DTOs.
4. **IPC surface (read-only):** one command `get_snapshot`; a `notify` file-watcher emits a
   `store-changed` event. Mutation commands, window flags, and file-drop are deferred.
5. **Frontend = read-only shell** (Radix defaults only): two-pane layout, sidebar + detail +
   task list display, selection + search + incomplete-only filter client-side, live-reload
   on `store-changed`.
6. **State:** re-fetch the snapshot on mount and on every `store-changed`; hold it + view
   state (selected collection, search text, filters) in React. `pond-core` is the single
   source of truth.
7. **Verification:** logic + build gates + manual launch; NO visual-regression tests (§7).

## 3. Architecture

```
rust-toolchain.toml            channel = "1.96.0"
src-tauri/                     pond-tauri crate (workspace member)
├─ Cargo.toml                  deps: tauri 2, pond-core, serde/serde_json, notify (no Tauri plugins needed in Phase 3; clipboard/dialog/fs plugins come in Phases 4–5)
├─ tauri.conf.json             window + dev-server (Vite) / frontendDist config
├─ capabilities/default.json   Tauri v2 capabilities (allow the get_snapshot command + event)
├─ build.rs
└─ src/
   ├─ main.rs                  Tauri builder: register command, start watcher, run
   ├─ dto.rs                   SnapshotDto / CollectionSummaryDto / CollectionGroupSummaryDto (Serialize, camelCase)
   ├─ commands.rs              #[tauri::command] get_snapshot -> SnapshotDto
   └─ watcher.rs               notify watcher on the store dir -> emit "store-changed"
src/                           frontend (Vite root)
├─ main.tsx                    React entry, <Theme> wrapper, import @radix-ui/themes/styles.css
├─ App.tsx                     two-pane layout, snapshot fetch + store-changed subscription, view state
├─ api/
│  ├─ client.ts                typed invoke wrapper: getSnapshot(); onStoreChanged(cb)
│  └─ types.ts                 TS types mirroring the DTOs
├─ state/
│  └─ view.ts                  pure view-model logic: filter/sort/group (unit-tested)
└─ components/
   ├─ Sidebar.tsx              All row + Collapsible groups + collection rows
   ├─ DetailPane.tsx           header (title/subtitle + search) + task list
   └─ TaskRow.tsx              status indicator + title + note line + collection chip (display-only)
index.html, vite.config.ts, package.json, tsconfig.json
```

Each frontend unit is focused: `api/` is the only place that touches `invoke`; `state/view.ts`
is pure (snapshot + view-state → what to render) and is where the testable logic lives;
components are thin renderers.

## 4. IPC

- **Command:** `get_snapshot() -> SnapshotDto` where
  `SnapshotDto { items: Vec<TaskItem>, collections: Vec<CollectionSummaryDto>, groups: Vec<CollectionGroupSummaryDto> }`.
  - `CollectionSummaryDto { name, displayName, groupName, totalCount, incompleteCount, statusIndicator: Option<String>, color: String, isArchived, promptTemplate: Option<String> }` (camelCase; `color`/`statusIndicator` as rawValues).
  - `CollectionGroupSummaryDto { name, collections: Vec<CollectionSummaryDto> }`.
  - Built from `TaskStore::open_default()` → `items()` / `collection_summaries()` / `collection_group_summaries()`. Errors map to a string the frontend can show.
- **Event:** a `notify` watcher on the store directory (debounced, mirroring the Swift
  `StoreChangeMonitor`'s ~100 ms settle) emits `store-changed` to the window; the frontend
  re-invokes `get_snapshot`.
- **Capabilities:** Tauri v2 requires explicit capabilities; the default capability allows
  exactly `get_snapshot` (+ the event), nothing more.

## 5. Frontend read-only shell (Radix defaults only)

- `<Theme>` with no props (default accent/gray/radius/scaling; system light/dark); import
  `@radix-ui/themes/styles.css`. Icons from `@radix-ui/react-icons`.
- **Layout:** `Flex`/`Box` two-pane (fixed-width sidebar + flexible detail).
- **Sidebar:** an "All" row (incomplete-count `Badge`); `Collapsible` group sections; each
  collection row a full-width left-aligned `Button` (`variant="soft"` when selected, else
  `ghost`) with a `Text color={color}` dot icon + name + incomplete-count `Badge`; archived
  collections rendered muted (only when a show-archived view-state is on — default off).
- **DetailPane:** header `Heading` (selected collection's display name) + `Text` subtitle
  (group, when not the default group) + a `TextField` search with a search-icon slot.
- **TaskRow (display-only):** a colored status indicator (Radix icon in the status's color),
  the title `Text` (dimmed for in-progress/completed), a note line (icon + body) when a note
  exists, and a collection `Badge` chip shown only on the "All" view.
- **Filtering (client-side, read-only):** selecting a collection filters items to it; search
  filters by title/collection/id/note; an incomplete-only toggle hides completed. These live
  in `state/view.ts` as pure functions over the snapshot + view-state.

## 6. Data flow

1. On mount, `App` calls `client.getSnapshot()` → stores the snapshot in React state.
2. `client.onStoreChanged(() => refetch())` subscribes to the Tauri event; any store
   change (incl. a `taskpond` CLI edit) triggers a re-fetch.
3. View state (selected collection id, search text, filter toggles) is React state; the
   rendered sidebar/list are derived by `state/view.ts` from snapshot + view-state.
4. No writes in Phase 3 — the UI is a pure projection of `pond-core`.

## 7. Verification (no visual-regression tests)

- **Rust unit tests** (`pond-tauri`): the DTO mapping (`CollectionSummary` → DTO) and the
  watcher's debounce/emit logic (test the watcher module against a temp dir + a channel,
  not a live window).
- **Frontend logic tests** (Vitest): `state/view.ts` behavior — e.g., "given snapshot +
  selected collection + search, the visible items are X", group ordering, incomplete-only
  filtering. These test **behavior**, not DOM structure or pixels, so they survive design
  tweaks. The typed `client.ts` is tested against a mocked `invoke`.
- **Build gates:** `cargo build` (workspace, on the pinned toolchain) and `vite build` +
  `tsc --noEmit` (frontend typecheck) must pass clean; `cargo clippy`/`cargo fmt` stay clean.
- **Manual launch (you):** `cargo tauri dev` once to confirm the window renders real data and
  live-reloads when a `taskpond` CLI edit changes the store. This human check replaces any
  automated visual test.
- Explicitly **out of scope:** screenshot/visual-regression baselines, DOM-snapshot tests.

## 8. References
- Overall migration spec — `docs/superpowers/specs/2026-06-02-tauri-radix-migration-design.md` (§3 architecture, §6 Radix mapping).
- `pond-core` public API (`get_snapshot` sources): `items`, `collection_summaries`, `collection_group_summaries`.
- Swift behavioral reference for the read-only shell: `Sources/PondApp/SidebarView.swift`, `DetailView.swift`, `TaskRow.swift`, `StoreChangeMonitor.swift`.
- Tauri v2 docs (commands, events, capabilities); Radix Themes docs.
