![Pond screenshot](docs/screenshot.png)

# Pond

A small macOS task app (Tauri + React + Radix Themes) with a shared `taskpond`
command-line interface. The GUI and the CLI use the same on-disk JSON store, so an edit
from either side shows up live in the other.

## Desktop app

Prerequisites: the pinned Rust toolchain (installed automatically from `rust-toolchain.toml`),
Node + npm, and the Tauri CLI.

```sh
cargo install tauri-cli --version "^2"   # once (or use: npx @tauri-apps/cli)
(cd frontend && npm install)
cargo tauri dev                          # launch with hot-reload against the Vite dev server
```

- **Dev (hot-reload):** `cargo tauri dev`.
- **Bundle:** `cargo tauri build` — produces the `.app`, with the `taskpond` CLI bundled
  alongside the executable as a sidecar.

The window supports full editing: create, rename, delete, status changes, notes, inline
title/note editing (split/merge), collection and group management, per-collection prompts,
export, and file-drop-to-create. Edits made by the `taskpond` CLI appear live via the file
watcher. The store path honors `POND_STORE`.

**Keyboard shortcuts:**
- `Cmd+N` — create a new task in the selected collection.
- `Cmd+Backspace` — delete the focused task.
- `Enter` (in title editor) — split at caret; `Cmd+Enter` — confirm title.
- `Backspace` at start of a title — merge into the previous task (if it is a note-free draft or ready task).
- `Esc` — discard edits.

## CLI

```sh
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

`taskpond item update --status` requires one status: `ready`, `draft`, `in-progress`,
`completed`, `on-hold`, `rejected`, or `aborted`.
`taskpond item update` changes an existing item in place without changing its id.
Successful non-help CLI commands write JSON to standard output. Item output includes `id`,
`status`, `collection`, `title`, and an optional `note`.
The default collection is `Inbox`. Collections outside the default group (`No Group`) are
addressed as `<group>/<collection>`.

Examples:

```sh
taskpond item create "Buy milk"
taskpond item create --collection Projects/Work "Draft proposal"
taskpond item get
taskpond item get -s ready -c Inbox
taskpond item update 1a2b3c4d --collection Errands -s ready "Buy oat milk"
taskpond item update 1a2b3c4d --status completed
taskpond item update 1a2b3c4d --status in-progress
taskpond item update 1a2b3c4d --status on-hold
taskpond item update 1a2b3c4d --status aborted
taskpond item delete 1a2b3c4d
taskpond item delete --collection Inbox
taskpond collection list
taskpond collection create Errands
taskpond collection rename Errands Personal
taskpond collection color Personal blue
taskpond collection clear Personal --completed
taskpond collection delete Personal
```

The desktop app's **Settings → Command** tab installs a `taskpond` symlink at
`~/.local/bin/taskpond` pointing at the CLI bundled alongside the app, and shows a
`PATH` hint when `~/.local/bin` is not on your `PATH`.

## Toolchain

- **Rust 1.96.0** — pinned in `rust-toolchain.toml` (rustup installs it automatically for this repo; the global default is untouched). Crates use edition 2021.
- **Tauri v2** — `tauri` / `tauri-build` 2.x, plus the `clipboard-manager` and `dialog` plugins. Install the CLI once: `cargo install tauri-cli --version "^2"` (or use `npx @tauri-apps/cli`).
- **Frontend** — Node with **npm**; Vite 5, React 18, TypeScript 5, `@radix-ui/themes` 3, `@tauri-apps/api` 2; logic tests via Vitest 2.
- `taskpond` ships as a Tauri **sidecar** (`externalBin`); `Scripts/build-sidecar.mjs` builds it and stages `src-tauri/binaries/taskpond-<target-triple>` before `cargo tauri dev`/`build`.

## Workspace layout

```text
crates/pond-core      data store + domain logic (also hosts the macOS `cli_install` module)
crates/taskpond-cli   the `taskpond` CLI binary (clap)
src-tauri/            the Tauri app: Rust commands over pond-core + the file watcher
frontend/             the React + Radix Themes frontend (Vite)
```

Tests: `cargo test` (Rust workspace) and `(cd frontend && npm test)` (frontend logic, Vitest).
Run the CLI directly with `cargo run -p taskpond-cli -- item get`. The store path honors `POND_STORE`.
