# Phase 2: `taskpond` CLI + macOS Install Feature — Design

- **Date:** 2026-06-02
- **Status:** Approved design
- **Phase:** 2 of 5 (see `2026-06-02-tauri-radix-migration-design.md`)
- **Builds on:** Phase 1 `pond-core` (complete: domain model + file-locked store, 66 tests).

## 1. Overview

Port the Swift `taskpond` CLI (`Sources/TaskCLI/main.swift`) and the macOS command-line
installer (`Sources/TaskCore/CommandLineInstaller.swift`) to Rust, on top of the
completed `pond-core`. The CLI is cross-platform (it only drives `pond-core`); the install
feature is Unix/macOS-specific.

### Goals
- A `taskpond` binary whose command surface, stdout JSON, exit codes, and argument
  behaviors match the Swift CLI.
- A tested installer library (port of `CommandLineInstaller`) that the GUI will drive in
  Phase 5.

### Non-goals (Phase 2)
- A CLI `install`/`uninstall` subcommand (install stays GUI-driven, matching Swift).
- Windows install support (the CLI commands still run on Windows; only the symlink
  installer is Unix-only).
- The final production install *target* path (depends on the Tauri bundle layout — Phase 3+).

## 2. Confirmed decisions (from brainstorming)

1. **Library-only installer, exact CLI parity.** The `taskpond` command surface matches
   Swift exactly — no install subcommand. The installer is a library, exercised by unit
   tests in Phase 2; the GUI drives it in Phase 5.
2. **Installer lives in `pond-core`** as a `cli_install` module (mirrors Swift's
   `CommandLineInstaller` in `TaskCore`; reuses `pond-core`'s data-dir; `pond-tauri` gets it
   for free via its existing `pond-core` dependency). It is OS-integration but has no UI
   dependency.
3. **`clap` for parsing, with parity on the machine contract.** stdout JSON shapes, exit
   codes, command/flag surface, `\n`/`\r`/`\t`/`\\` unescaping, the "`--collection` XOR ids"
   rule, and `status`/`color` string parsing all match Swift. **Accepted divergence:**
   human-facing arg-error and `--help` text follow clap conventions, not Swift's exact
   `CLIError` strings (not a machine contract).
4. **Installer is `cfg(unix)`, macOS-primary** (Linux works as a bonus — same
   `~/.local/bin` symlink). Windows install out of scope.
5. **Production install target deferred to packaging.** The default target resolver is
   best-effort (`current_exe`-based) for now; finalized when the Tauri bundle exists. Phase 2
   fully builds + tests install/uninstall/status/record logic via injectable paths.

## 3. Architecture

```
crates/
├─ pond-core/          (Phase 1, complete) + NEW module:
│  └─ src/cli_install.rs   Installer (symlink + record), cfg(unix)
└─ taskpond-cli/        NEW bin crate — the `taskpond` CLI (clap), depends on pond-core
   └─ src/
      ├─ main.rs            clap command tree + dispatch
      ├─ output.rs          ItemOutput / CollectionOutput (CLI stdout structs)
      └─ parse.rs           arg helpers: unescape, status/color parsing, XOR target
```

- `taskpond-cli` depends only on `pond-core` (+ `clap`, `serde`).
- `cli_install` is added to `pond-core` and re-exported (`pond_core::cli_install::Installer`,
  `InstallStatus`).

## 4. CLI command surface → `pond-core` mapping

Exact surface (see `README.md` CLI section and `main.swift`):

| Command | `pond-core` call |
|---|---|
| `item create [-c <col>] <title…>` | `store.add(title, col?, None, false, Ready)` |
| `item get [-s <status>] [-c <col> \| <id…>]` | `store.items(status?, col?, ids, None)` |
| `item update <id> [-c <col>] [-s <status>] [<title…>]` | `store.update(id, title?, col?, status?)` |
| `item note add <id> --body <body>` | `store.add_note(id, body)` |
| `item note update <id> --body <body>` | `store.update_note(id, body)` |
| `item note delete <id>` | `store.delete_note(id)` |
| `item delete <-c <col> \| <id…>>` | `store.delete_many(ids, col?)` |
| `collection list` | `store.collection_summaries()` |
| `collection create <name>` | `store.create_collection(name, DEFAULT_GROUP)` |
| `collection rename <old> <new>` | `store.rename_collection(old, new)` |
| `collection color <name> <color>` | `store.set_collection_color(name, color)` |
| `collection delete <name>` | fetch summary, then `store.delete_collection(name)` |
| `collection clear <name> [--completed]` | `store.clear_items(name, completed_only)` |

- The store is constructed per-invocation with the default path or the `POND_STORE` override
  (`pond-core::paths`).
- `status` strings (`ready`, `draft`, `in-progress`, `completed`, `on-hold`, `aborted`,
  `rejected`) and `color` strings (`gray`…`purple`) parse via the serde rawValues (a small
  `parse.rs` helper; invalid values produce the Swift-equivalent "Expected one of …" message).
- Title/note/name positional args are `\n`/`\r`/`\t`/`\\`-unescaped (port of Swift
  `cliUnescaped`); `--` takes the remainder as the title.

### Output (stdout JSON — exact Swift shapes)

`pond-core`'s `to_pretty_sorted` (pretty, sorted keys) with **no date fields** (CLI output
omits timestamps):

- **`ItemOutput`**: `id`, `status` (rawValue), `collection`, `title`, `note` (optional
  `{ id, version, body }`).
- **`CollectionOutput`**: `name`, `totalCount`, `incompleteCount`, `color` (rawValue),
  `statusIndicator` (optional rawValue).

These are CLI-local structs in `taskpond-cli/src/output.rs` (Swift kept them private to the
CLI), built from `pond-core` `TaskItem` / `CollectionSummary`.

### Errors & exit codes
- Success: print JSON to stdout, exit 0.
- Domain failure: print `StoreError`'s parity message to stderr, exit 1 (covers
  not-found / ambiguous-id / collection-conflict / invalid-title / no-matching-tasks /
  MissingTarget / TargetConflict — these come straight from `pond-core`, already parity).
- Bespoke arg validations that are behavioral (e.g. "create requires a title") are
  replicated in handlers; purely structural parse errors (unknown flag, missing arg) use
  clap's format.

## 5. Installer (`pond-core::cli_install`)

Faithful port of `CommandLineInstaller` (`cfg(unix)`):

- **`Installer { link_path, target_path, record_path }`** — all injectable (tests pass temp
  paths). Default constructor: `link_path = ~/.local/bin/taskpond`,
  `record_path = <data-dir>/cli-install.json`, `target_path = <best-effort current_exe>`
  (TODO marker: finalize for the Tauri bundle in packaging).
- **`InstallStatus { link_path, target_path, installed, conflict_description,
  install_directory_is_in_path, can_uninstall, can_install }`** (`can_install` derived).
- **`status()`** — classify the link path: missing / non-symlink file (conflict) /
  symlink (installed if it points at target; else conflict, removable only if Pond created
  it per the record); compute `install_directory_is_in_path` from `$PATH`.
- **`install()`** — require an executable target; create `~/.local/bin`; create/replace the
  symlink (replacing only a Pond-created symlink, else conflict error); write the record.
- **`uninstall()`** — remove the symlink (only if Pond-created) and the record.
- **`path_hint()`** — `export PATH="$HOME/.local/bin:$PATH"`.
- **Record** `cli-install.json` (`{ linkPath, targetPath, installedAt }`) via
  `pond-core`'s persisted JSON (ISO-8601, sorted).

The conflict/record logic mirrors Swift's `linkKind` + `canRemoveSymlink` (only remove a
symlink that points at our target or matches the recorded target).

## 6. Testing

- **Installer tests** (in `pond-core`): construct `Installer` with injected temp
  `link/target/record` paths under a `tempdir`; assert install → status(installed) →
  uninstall round-trips, and conflict handling (pre-existing file, foreign symlink). Port
  of `CommandLineInstallerTests`.
- **CLI tests** (in `taskpond-cli`): the crate exposes a testable in-process entry point
  `run(args: &[String], store: &TaskStore, out: &mut impl Write) -> Result<(), CliError>`
  (with `i32` exit mapping); `main.rs` is a thin wrapper that builds the store from
  `POND_STORE`/default, calls `run`, prints, and maps the result to an exit code. Tests call
  `run` directly against a `TaskStore` on a `tempdir` path, asserting captured stdout JSON
  and the returned result for each command — covering create/get/update/note/delete and
  collection list/create/rename/color/delete/clear, plus the XOR-target and
  invalid-status/color error paths. Port of the CLI behaviors in `TaskCLI`.

Both crates keep `cargo clippy -- -D warnings` clean and `cargo fmt`-clean, per the Phase 1
conventions.

## 7. References (source of truth)
- `Sources/TaskCLI/main.swift` — command surface, `ArgumentScanner`, output structs,
  `CLIError`, `cliUnescaped`.
- `Sources/TaskCore/CommandLineInstaller.swift` — installer logic, `linkKind`, record.
- `README.md` — CLI usage section (authoritative command list + examples).
- `crates/pond-core/` — the public API the CLI consumes.
- Overall migration spec — `docs/superpowers/specs/2026-06-02-tauri-radix-migration-design.md`.
