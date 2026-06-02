# Pond: SwiftUI → Tauri + Radix Themes Migration

- **Date:** 2026-06-02
- **Status:** Approved design (north-star spec)
- **Topic:** Port the Pond macOS task app from SwiftUI to Tauri (Rust + React/Radix Themes), with cross-platform (Linux) intent.

## 1. Overview

Pond is a small task manager: a collections/groups sidebar and a task list with a
7-state status model, one optional note per task, inline editing (including
merge/split), search, filters, archived collections, per-collection colors, and a
shared `taskpond` CLI. Today it is SwiftUI + a shared `TaskCore` Swift library.

We are rewriting it on **Tauri v2** with a **Rust core** shared by the GUI and a new
Rust CLI, and a **React + TypeScript + Radix Themes** frontend. Motivation: SwiftUI
friction and future Linux support.

### Goals

- Near-full feature parity with the current app **except drag-reordering** (items,
  collections, and groups reordering are deferred).
- A shared, well-tested Rust core reusable by the GUI, the CLI, and future platforms.
- A cross-platform `taskpond` CLI matching today's command surface, plus the macOS
  install/uninstall feature.
- A frontend built **only from stock Radix Themes** components — no theme
  customization, no custom CSS, no custom colors.

### Non-goals (v1)

- Drag-and-drop reordering of items, collections, or groups.
- Reading or migrating the existing Swift app's data (fresh start).
- Multi-window (the Swift app supports several windows; v1 ships one main window).
- Pixel-exact reproduction of native micro-animations.

## 2. Confirmed decisions

1. **Scope:** near-full parity, minus reordering.
2. **CLI:** build the Rust `taskpond` CLI **and** the macOS install feature.
3. **Data:** fresh start — no migration; new clean schema (`version: 1`).
4. **Color policy:** use **Radix's built-in named colors** via component `color` props
   for collection dots and status indicators. No custom CSS/hex, no theme
   `accentColor`/`radius`/`scaling` changes. This preserves the color-coding feature
   while honoring "default Radix only."
5. **Default-collection cleanup (enabled by fresh start):** the default collection is
   **`Inbox`** everywhere (store, CLI, GUI); a single internal sentinel group renders
   as **"No Group"**. Collections in that group are addressed by bare name; others as
   `Group/Collection`. The legacy `DefaultCollection`/`Collections` aliases are dropped.
6. **Store location (fresh dir):** resolved cross-platform via the `directories` crate —
   macOS `~/Library/Application Support/pond/tasks.json`, Linux
   `~/.local/share/pond/tasks.json`, Windows `%APPDATA%\pond\tasks.json`. The
   `POND_STORE` env override is kept (tests/CLI). `pond-core` owns this path so CLI and
   GUI always agree.
7. **Single window** for v1; multi-window noted as a later option.

## 3. Architecture

Cargo workspace + a Vite/React frontend, packaged by Tauri v2.

```
pond/
├─ crates/
│  ├─ pond-core/      Rust lib: domain model, JSON store (file-locked, atomic),
│  │                  validation/normalization, collections/groups, status rules,
│  │                  merge/split, prompt templates, export. Ported TaskCore. No UI deps.
│  ├─ taskpond-cli/   Rust bin: `taskpond` CLI (clap) + macOS install/uninstall.
│  └─ pond-tauri/     Rust bin: Tauri app; #[command] layer over pond-core,
│                     notify file-watcher → events, window flags, file-drop.
├─ src/               Frontend: React + TS + @radix-ui/themes (default <Theme>),
│                     @radix-ui/react-icons, typed invoke client, components.
├─ src-tauri/         Tauri config, capabilities, icons, CLI sidecar (pond-tauri here).
└─ Cargo.toml         workspace
```

- `pond-core` has no Tauri/UI dependency → unit-testable in isolation; the CLI and any
  future platform reuse it.
- `pond-tauri` owns OS integration that used to be AppKit: a `notify` watcher on the
  store directory emitting a `store-changed` event (replaces `StoreChangeMonitor`),
  always-on-top via the window API, and file-drop-to-create.
- The frontend holds **view state only** (selection, what's being edited, search text,
  filter toggles). All task/collection truth comes from `pond-core` via `invoke`.
- **Data flow:** mutations return a fresh snapshot; the UI re-renders from it (mirrors
  the current `reload()`-after-every-op pattern). Optimistic updates only added later if
  typing latency demands it.

## 4. Data model & store

### Domain types (ported from `TaskCore`)

- **`TaskItem`**: `id` (8-char hex), `version` (12-char random; bumped on every change for
  optimistic-concurrency `ifCurrent` checks), `title`, `collection`, `notes` (0 or 1
  `TaskNote`), `status`, `createdAt`, `updatedAt`.
- **`TaskNote`**: `id`, `version`, `body`, `createdAt`, `updatedAt`.
- **`TaskStatus`** (7): `draft`, `ready`, `in-progress`, `completed`, `on-hold`,
  `rejected`, `aborted`. `isIncomplete` = not `completed`.
- **`CollectionColor`** (7): `gray`, `red`, `orange`, `yellow`, `green`, `blue`, `purple`
  (map 1:1 onto Radix color names).
- **`CollectionSummary`**: `name`, `displayName`, `groupName`, `totalCount`,
  `incompleteCount`, `statusIndicator`, `color`, `isArchived`, `promptTemplate`.
- **`CollectionGroupSummary`**: `name`, `collections[]`.
- Roll-up `statusIndicator` precedence: `aborted` ▸ `rejected` ▸ `on-hold` (else none),
  computed in core. Counts computed in core.

### On-disk store

- One JSON file, same logical shape as today:
  `{ version: 1, collections[], collectionGroups[], collectionColors{}, collectionPrompts{}, archivedCollections[], items[] }`.
- ISO-8601 dates, sorted keys, pretty-printed.
- **Fresh start** removes all legacy migration machinery (`version < 6` paths, version
  probes, `DefaultCollection`/`Collections` aliasing). The new format reads only itself.

### Concurrency & safety (mirrors `withFile`)

- Advisory file lock (`fd-lock`/`fs2`) around every read/write.
- Atomic writes via temp-file + rename.
- This is what lets the GUI and CLI safely share the file.

### App settings (were `UserDefaults`/`@AppStorage`)

- `usesAutoDraft` (default on), `alwaysOnTop` (default off), `defaultPromptTemplate`
  (empty → built-in app default), last-selected collection.
- Stored in a small `settings.json` in the OS config dir.

## 5. IPC command surface (Tauri commands over `pond-core`)

Mutations return a fresh snapshot so the UI re-renders from Rust truth.

- **Snapshot:** `get_snapshot()` → `{ items, collectionSummaries, collectionGroupSummaries }`.
- **Items:** `create_item`, `update_item` (title/collection/status, `ifCurrent` version
  check), `set_status`, `move_item`, `delete_item`, `add_note`/`update_note`/`delete_note`,
  `merge_item`, `split_item`, `set_statuses` (bulk). *(reorder omitted.)*
- **Collections:** `create_collection`, `rename_collection`, `move_collection`,
  `set_collection_color`, `set_collection_archived`, `set_collection_prompt`,
  `delete_collection`, `delete_empty_collection`, `clear_items`, `export_collection`
  (returns JSON/JSONL text; frontend saves via Tauri dialog).
- **Groups:** `create_group`, `rename_group`, `delete_group`, `merge_group`. *(reorder omitted.)*
- **Prompts:** `collection_prompt_text` (template evaluated with `{{cliCommand}}` filled),
  `collection_cli_command`.
- **Settings:** `get_settings`, `set_setting`.
- **CLI installer:** `cli_install_status`, `cli_install`, `cli_uninstall`.
- **Window/OS:** `set_always_on_top`; file-drop via Tauri drag-drop event → `create_item`.
- **Events (Rust→JS):** `store-changed` from the `notify` watcher → frontend re-fetches.
  Clipboard (Copy ID / Prompt / CLI Command) via the Tauri clipboard plugin.

## 6. GUI structure & Radix mapping

Layout mirrors today's app, built only from stock Radix Themes parts wrapped in a default
`<Theme>` (no `accentColor`/`radius`/`scaling`/CSS overrides; system light/dark via default
`appearance`). Icons from `@radix-ui/react-icons`; where a status glyph has no exact Radix
equivalent, pick the closest stock icon. Radix `size` props (a built-in option, not
"customization") are used to pick the most compact stock sizes.

| Area | Behavior (parity) | Radix building blocks | v1 |
|---|---|---|---|
| Shell | sidebar + detail two-pane | `Flex`/`Box` layout primitives | ✅ |
| Sidebar rows | All / groups / collections, selection highlight, counts, color dots, archived (muted) | full-width `Button` (`soft` selected / `ghost` otherwise), `Badge` count, `Text color={color}` dot icon | ✅ |
| Collapsible groups | expand/collapse | `Collapsible` / `Accordion` | ✅ |
| Detail header | title + group subtitle, options menu, search | `Heading`/`Text`, `DropdownMenu`, `TextField` w/ search icon slot | ✅ |
| Task row | colored status icon, options menu, title, note line, collection chip on "All" | `IconButton` (`color` per status), `DropdownMenu`, `Text`, `Badge` | ✅ |
| Inline edit | click-to-edit, autosave, Enter/Tab/Esc/Cmd+Enter/arrows, merge/split, empty-deletes | `TextArea` (auto-grow) + React key handlers mirroring `TaskRow.swift` | ✅ ⚠️ |
| Status / collection / item context menus | exact items from the Swift menus | `ContextMenu` + `DropdownMenu` | ✅ |
| Sidebar footer menu | Hide Completed, Show Archived, Automatic Drafts, Always On Top, Settings | `DropdownMenu` with checkbox items | ✅ |
| Bulk status change | per-status → replacement grid | `Dialog` + `Select` rows | ✅ |
| Prompt editor / Delete confirm / Error | sheets & alerts | `Dialog`, `AlertDialog` | ✅ |
| Settings | Command / Prompt / System Information | `Dialog` + `Tabs` | ✅ |
| **Reorder** items/collections/groups (drag) | — | — | ❌ deferred |

## 7. Behavior parity notes

The Swift sources are the behavioral source of truth; implementation plans encode exact
values. Key behaviors to reproduce:

- **Status leading-click:** left-click on the status icon advances per
  `TaskStatus.leadingStatusClickTarget` (TaskViewSupport.swift); right-click sets `draft`.
  In-progress / completed tasks lock inline title & collection editing (note still editable).
- **Inline title editing:** click-to-edit; 500 ms debounced autosave (skipped during IME
  composition); `Enter` splits at caret (or creates a task below); `Cmd+Enter` confirms and
  moves to note/next; `Tab` saves → note or next; `Esc` discards; `End` to end; plain
  `↑/↓` move focus between rows; `Backspace` at the empty start merges into the previous
  task (only if previous is draft/ready with no note); empty title on commit deletes the task.
- **Note editing:** click-to-edit; autosave; plain `Return`/`Tab` move to next; `Esc`
  discards; empty note deletes the note on save.
- **Auto-draft (`usesAutoDraft`, default on):** edited tasks drop to `draft`; confirmation
  promotes to `ready`. See `autoDraftEditStatus` / `autoDraftConfirmationStatus`.
- **Completed-item visibility:** when "Hide Completed Items" is on, a just-completed item
  stays visible only for its fade-out (~0.22 s) before hiding — best-effort with CSS.
- **Global shortcuts:** `Cmd+N` new draft; `Cmd+⌫` delete focused task; `Cmd+D` set draft;
  `Cmd+⌥N` focus/create note; `PageUp/PageDown` and `Cmd+Opt+↑/↓` switch collection;
  `Cmd+⌫` (app menu) delete selected collection.
- **Prompt templates:** per-collection template overrides the app default (which overrides
  the built-in `applicationDefaultTemplate`). `{{cliCommand}}` resolves to the collection's
  `taskpond item get -c <collection>` command. "Copy Prompt" copies the evaluated template;
  "Copy CLI Command" copies the command alone. (TaskPromptTemplate.swift / TaskPromptSettings.swift)
- **Export:** per-collection, `JSON` (pretty, includes `collection`/`exportedAt`/`items`) or
  `JSONL` (one item per line). (CollectionExport.swift)

## 8. CLI parity (Phase 2)

Replicate exactly (see TaskCLI/main.swift, README.md):

```
taskpond item create [-c|--collection <collection>] <title...>
taskpond item get [-s|--status <status>] [-c|--collection <collection> | <id...>]
taskpond item update <id> [-c|--collection <collection>] [-s|--status <status>] [<title...>]
taskpond item note add <id> --body <body>
taskpond item note update <id> --body <body>
taskpond item note delete <id>
taskpond item delete <-c|--collection <collection> | <id...>>
taskpond collection list
taskpond collection create <name>
taskpond collection rename <old-name> <new-name>
taskpond collection color <name> <gray|red|orange|yellow|green|blue|purple>
taskpond collection delete <name>
taskpond collection clear <name> [--completed]
```

- Same stdout JSON shapes (`ItemOutput` with `id`/`status`/`collection`/`title`/`note`;
  `CollectionOutput` with `name`/`totalCount`/`incompleteCount`/`color`/`statusIndicator`),
  pretty + sorted keys, no date fields in CLI output.
- Same `\n`/`\r`/`\t`/`\\` unescaping of title/note/name arguments.
- Same error messages and non-zero exit on failure.
- **Installer:** symlink `~/.local/bin/taskpond` → bundled binary; record file
  (`cli-install.json`) in the app data dir; status detection (missing / conflicting file /
  conflicting symlink / installed); `export PATH="$HOME/.local/bin:$PATH"` hint. The target
  binary ships as a Tauri **sidecar/resource**; the installer points the symlink at the
  bundled path (replacing the Swift `Contents/Library/Helpers` inference).

## 9. Phasing

Dependency order: **1 → (2 ∥ 3) → 4 → 5**. Each phase is its own implementation plan with
its own verification.

1. **`pond-core` (Rust lib).** Domain types, file-locked + atomic JSON store, validation/
   normalization, collections/groups logic, status rules, merge/split, prompt-template eval,
   export encoding. Port `TaskStoreTests`. *Verify:* `cargo test` green.
2. **`taskpond` CLI + installer.** clap CLI per §8; stdout/unescape/error parity; macOS
   install/uninstall. Port `CommandLineInstallerTests`. *Verify:* CLI parity tests; install
   round-trip.
3. **Tauri shell + IPC + Radix scaffold.** Tauri v2 + Vite/React/TS, default `<Theme>`,
   command layer, typed invoke client, `store-changed` watcher, two-pane shell, snapshot
   fetch + **read-only** render (sidebar, list, selection, search/filter). *Verify:* launches
   on real data; live-reloads on CLI edits.
4. **Core management.** Status change (click + menu), create/delete tasks, note display,
   collection/group CRUD + color + archive + show-archived + counts + All view, `Cmd+N`,
   delete shortcut, inline title/note editing (no merge/split yet). *Verify:* day-to-day usable.
5. **Advanced parity & integration.** Merge/split-on-edit, auto-draft, full keyboard map,
   bulk-status dialog, prompts (edit / copy prompt / copy CLI command), export save dialog,
   always-on-top, file-drop-to-create, Settings (CLI install UI + default prompt + system
   info), error/confirm dialogs. *Verify:* feature-by-feature vs. the Swift app.

## 10. Risks

- **Inline editor (highest risk):** caret/IME/split/merge on a `<textarea>` re-creating
  `NSTextView` behavior — isolated to Phase 5, ported carefully; micro-animations best-effort.
- **Visual density:** stock Radix defaults are airier than the tight native list; the app
  will read more "web app" than native — a direct consequence of the no-customization
  constraint (mitigated only by stock `size` props).
- **Icons:** SF Symbols → closest `@radix-ui/react-icons`; some status glyphs approximate.
- **CLI packaging:** depends on bundling the `taskpond` binary as a Tauri sidecar/resource
  and pointing the installer at the bundled path.

## 11. References (behavioral source of truth)

- Core: `Sources/TaskCore/TaskItem.swift`, `TaskStore.swift`, `TaskItemSupport.swift`,
  `TaskCollectionSupport.swift`, `TaskStoreError.swift`, `JSONCoding.swift`,
  `TaskPromptTemplate.swift`, `CommandLineInstaller.swift`.
- GUI: `Sources/PondApp/AppModel.swift`, `ContentView.swift`, `SidebarView.swift`,
  `DetailView.swift`, `TaskRow.swift`, `TaskViewSupport.swift`, `CollectionMenus.swift`,
  `InputEventHandlers.swift`, `SettingsView.swift`, `CollectionExport.swift`,
  `TaskPromptSettings.swift`, `StoreChangeMonitor.swift`, `WindowControllers.swift`.
- CLI: `Sources/TaskCLI/main.swift`. Tests: `Tests/TaskCoreTests/*`.
