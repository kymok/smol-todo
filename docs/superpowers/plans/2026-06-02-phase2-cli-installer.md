# Phase 2: `taskpond` CLI + Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `taskpond` CLI binary (faithful port of `Sources/TaskCLI/main.swift`) and a `cli_install` installer module in `pond-core` (port of `Sources/TaskCore/CommandLineInstaller.swift`), both on the completed Phase 1 `pond-core`.

**Architecture:** A new `crates/taskpond-cli/` package with a library (`run()` entry point, clap command tree, output structs, arg parsing) and a thin `taskpond` binary. The installer is a `cfg(unix)` module added to `pond-core`. The CLI is cross-platform; the installer is Unix/macOS. All persistence goes through `pond-core`'s `TaskStore`.

**Tech Stack:** Rust 2021, `clap` (derive), `serde`/`serde_json`, `pond-core` (path dep), `tempfile` (dev), `chrono` (installer record).

**Conventions (every task):**
- Run `cargo test -p <crate>` from the repo root; final task runs the whole workspace.
- After tests pass: `cargo fmt` and `cargo clippy -p <crate> -- -D warnings` before committing.
- All `use` statements at the TOP of each file (module-level and inside `mod tests`).
- The toolchain is Rust 1.72-era; do not add dependencies beyond those listed in Task 1.
- The Swift sources (`Sources/TaskCLI/main.swift`, `Sources/TaskCore/CommandLineInstaller.swift`) and `README.md` are the behavioral source of truth.

---

## File Structure

```
crates/taskpond-cli/
├─ Cargo.toml              # lib + [[bin]] taskpond; deps clap, serde, serde_json, pond-core
└─ src/
   ├─ lib.rs              # pub fn run(...) + CliError + module wiring
   ├─ cli.rs              # clap Cli/Command enums + dispatch handlers
   ├─ output.rs           # ItemOutput / NoteOutput / CollectionOutput (stdout JSON)
   ├─ parse.rs            # unescape, parse_status, parse_color
   └─ main.rs             # thin wrapper: build store, call run(), map exit code
crates/pond-core/
└─ src/cli_install.rs     # Installer, InstallStatus, InstallError, record (cfg(unix))
Cargo.toml                # workspace: add crates/taskpond-cli to members
```

Responsibilities: `parse.rs` and `output.rs` are pure (no I/O); `cli.rs` holds the command dispatch (calls `pond-core`, writes JSON to a `&mut dyn Write`); `lib.rs` exposes the testable `run`; `main.rs` is the only place that touches the real store/stdout/exit. `cli_install` is self-contained OS-integration in `pond-core`.

---

## Task 1: Scaffold `taskpond-cli` crate

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/taskpond-cli/Cargo.toml`
- Create: `crates/taskpond-cli/src/lib.rs`
- Create: `crates/taskpond-cli/src/main.rs`

- [ ] **Step 1: Add the crate to the workspace** — set root `Cargo.toml` members to:

```toml
[workspace]
members = ["crates/pond-core", "crates/taskpond-cli"]
resolver = "2"

[workspace.dependencies]
getrandom = "=0.2.15"
tempfile = "=3.8.1"
```

- [ ] **Step 2: Create `crates/taskpond-cli/Cargo.toml`**

```toml
[package]
name = "taskpond-cli"
version = "0.1.0"
edition = "2021"

[lib]
name = "taskpond_cli"
path = "src/lib.rs"

[[bin]]
name = "taskpond"
path = "src/main.rs"

[dependencies]
pond-core = { path = "../pond-core" }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 3: Create `crates/taskpond-cli/src/lib.rs` with a smoke test**

```rust
//! The `taskpond` CLI: a thin command layer over `pond-core`.

#[cfg(test)]
mod smoke {
    #[test]
    fn crate_builds() {
        assert_eq!(2 + 2, 4);
    }
}
```

- [ ] **Step 4: Create `crates/taskpond-cli/src/main.rs`**

```rust
fn main() {
    // Replaced in Task 4 with the real entry point.
    println!("taskpond");
}
```

- [ ] **Step 5: Verify + commit**

Run: `cargo test -p taskpond-cli`
Expected: PASS (1 test). If `clap 4.x` resolves a version needing a newer Rust than 1.72, pin `clap = "=4.4.18"` (the last 4.x supporting Rust 1.72) and note it.

```bash
cargo fmt && cargo clippy -p taskpond-cli -- -D warnings
git add Cargo.toml Cargo.lock crates/taskpond-cli
git commit -m "feat(cli): scaffold taskpond-cli crate"
```

---

## Task 2: Argument parsing helpers (`parse.rs`)

**Files:**
- Create: `crates/taskpond-cli/src/parse.rs`
- Modify: `crates/taskpond-cli/src/lib.rs`

Port `cliUnescaped` and the status/color parsing. Invalid status/color produce the Swift-parity messages.

- [ ] **Step 1: Write the failing test** — create `crates/taskpond-cli/src/parse.rs`

```rust
use pond_core::{CollectionColor, TaskStatus};

/// Port of Swift `cliUnescaped`: \n \r \t \\ become their control chars; an
/// unknown escape keeps the backslash; a trailing backslash is preserved.
pub fn unescape(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut escaping = false;
    for ch in input.chars() {
        if escaping {
            match ch {
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                '\\' => result.push('\\'),
                other => {
                    result.push('\\');
                    result.push(other);
                }
            }
            escaping = false;
        } else if ch == '\\' {
            escaping = true;
        } else {
            result.push(ch);
        }
    }
    if escaping {
        result.push('\\');
    }
    result
}

const STATUS_HELP: &str =
    "Expected 'ready', 'draft', 'in-progress', 'completed', 'on-hold', 'aborted', or 'rejected'.";
const COLOR_HELP: &str = "Expected 'gray', 'red', 'orange', 'yellow', 'green', 'blue', or 'purple'.";

pub fn parse_status(value: &str) -> Result<TaskStatus, String> {
    match value {
        "draft" => Ok(TaskStatus::Draft),
        "ready" => Ok(TaskStatus::Ready),
        "in-progress" => Ok(TaskStatus::InProgress),
        "completed" => Ok(TaskStatus::Completed),
        "on-hold" => Ok(TaskStatus::OnHold),
        "rejected" => Ok(TaskStatus::Rejected),
        "aborted" => Ok(TaskStatus::Aborted),
        _ => Err(STATUS_HELP.to_string()),
    }
}

pub fn parse_color(value: &str) -> Result<CollectionColor, String> {
    match value {
        "gray" => Ok(CollectionColor::Gray),
        "red" => Ok(CollectionColor::Red),
        "orange" => Ok(CollectionColor::Orange),
        "yellow" => Ok(CollectionColor::Yellow),
        "green" => Ok(CollectionColor::Green),
        "blue" => Ok(CollectionColor::Blue),
        "purple" => Ok(CollectionColor::Purple),
        _ => Err(COLOR_HELP.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescape_handles_known_and_unknown_escapes() {
        assert_eq!(unescape(r"a\nb"), "a\nb");
        assert_eq!(unescape(r"a\tb"), "a\tb");
        assert_eq!(unescape(r"a\\b"), r"a\b");
        assert_eq!(unescape(r"a\qb"), r"a\qb"); // unknown escape keeps the backslash
        assert_eq!(unescape(r"trailing\"), r"trailing\");
    }

    #[test]
    fn status_parsing() {
        assert_eq!(parse_status("in-progress").unwrap(), TaskStatus::InProgress);
        assert_eq!(parse_status("ready").unwrap(), TaskStatus::Ready);
        assert!(parse_status("nope").unwrap_err().contains("Expected 'ready'"));
    }

    #[test]
    fn color_parsing() {
        assert_eq!(parse_color("purple").unwrap(), CollectionColor::Purple);
        assert!(parse_color("teal").unwrap_err().contains("Expected 'gray'"));
    }
}
```

- [ ] **Step 2: Wire the module** — set `crates/taskpond-cli/src/lib.rs` to:

```rust
//! The `taskpond` CLI: a thin command layer over `pond-core`.

pub mod parse;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p taskpond-cli parse`
Expected: PASS (3 tests).

- [ ] **Step 4: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p taskpond-cli -- -D warnings
git add crates/taskpond-cli/src/parse.rs crates/taskpond-cli/src/lib.rs
git commit -m "feat(cli): add arg parsing helpers"
```

---

## Task 3: Output structs (`output.rs`)

**Files:**
- Create: `crates/taskpond-cli/src/output.rs`
- Modify: `crates/taskpond-cli/src/lib.rs`

Port the Swift `ItemOutput` / `NoteOutput` / `CollectionOutput`. Enums serialize to their rawValues; `note` / `statusIndicator` are omitted when absent; keys are sorted via `pond_core::json::to_pretty_sorted`; no date fields.

- [ ] **Step 1: Write the failing test** — create `crates/taskpond-cli/src/output.rs`

```rust
use pond_core::{CollectionColor, CollectionSummary, TaskItem, TaskStatus};
use serde::Serialize;

#[derive(Serialize)]
pub struct NoteOutput {
    pub id: String,
    pub version: String,
    pub body: String,
}

#[derive(Serialize)]
pub struct ItemOutput {
    pub id: String,
    pub status: TaskStatus, // serializes to its rawValue (e.g. "in-progress")
    pub collection: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<NoteOutput>,
}

impl ItemOutput {
    pub fn from_item(item: &TaskItem) -> Self {
        ItemOutput {
            id: item.id.clone(),
            status: item.status,
            collection: item.collection.clone(),
            title: item.title.clone(),
            note: item.note.as_ref().map(|n| NoteOutput {
                id: n.id.clone(),
                version: n.version.clone(),
                body: n.body.clone(),
            }),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionOutput {
    pub name: String,
    pub total_count: usize,
    pub incomplete_count: usize,
    pub color: CollectionColor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_indicator: Option<TaskStatus>,
}

impl CollectionOutput {
    pub fn from_summary(summary: &CollectionSummary) -> Self {
        CollectionOutput {
            name: summary.name.clone(),
            total_count: summary.total_count,
            incomplete_count: summary.incomplete_count,
            color: summary.color,
            status_indicator: summary.status_indicator,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use pond_core::json::to_pretty_sorted;

    fn item() -> TaskItem {
        let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();
        let mut it = TaskItem::new("0123abcd".into(), "Buy milk".into(), "Inbox".into(), TaskStatus::InProgress, now);
        it.version = "v".repeat(12);
        it
    }

    #[test]
    fn item_output_omits_note_and_dates_uses_raw_status() {
        let json = to_pretty_sorted(&ItemOutput::from_item(&item())).unwrap();
        assert!(json.contains("\"status\": \"in-progress\""));
        assert!(!json.contains("note"));
        assert!(!json.contains("createdAt") && !json.contains("updatedAt"));
        // keys are sorted: collection before id before status before title
        let c = json.find("collection").unwrap();
        let t = json.find("title").unwrap();
        assert!(c < t);
    }

    #[test]
    fn collection_output_camel_case_and_raw_color() {
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
        let json = to_pretty_sorted(&CollectionOutput::from_summary(&summary)).unwrap();
        assert!(json.contains("\"incompleteCount\": 2"));
        assert!(json.contains("\"color\": \"blue\""));
        assert!(json.contains("\"statusIndicator\": \"on-hold\""));
    }
}
```

- [ ] **Step 2: Add `chrono` dev-dependency** — under `[dev-dependencies]` in `crates/taskpond-cli/Cargo.toml` add:

```toml
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 3: Wire the module** — append to `crates/taskpond-cli/src/lib.rs`:

```rust
pub mod output;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p taskpond-cli output`
Expected: PASS (2 tests).

- [ ] **Step 5: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p taskpond-cli -- -D warnings
git add crates/taskpond-cli
git commit -m "feat(cli): add stdout output structs"
```

---

## Task 4: `CliError`, clap tree, `run()` skeleton, `main.rs`

**Files:**
- Create: `crates/taskpond-cli/src/cli.rs`
- Modify: `crates/taskpond-cli/src/lib.rs`
- Modify: `crates/taskpond-cli/src/main.rs`

Define the command tree and the testable `run()` entry point. Handlers are stubs returning `Ok(())` for now (filled in Tasks 5–9), except the dispatch wiring and error type.

- [ ] **Step 1: Write the failing test** — create `crates/taskpond-cli/src/cli.rs`

```rust
use crate::output::{CollectionOutput, ItemOutput};
use crate::parse::{parse_color, parse_status, unescape};
use clap::{Parser, Subcommand};
use pond_core::json::to_pretty_sorted;
use pond_core::{StoreError, TaskStore, DEFAULT_COLLECTION, DEFAULT_GROUP};
use std::io::Write;

#[derive(Debug)]
pub enum CliError {
    Parse(clap::Error),
    Store(StoreError),
    Usage(String),
}

impl From<StoreError> for CliError {
    fn from(e: StoreError) -> Self {
        CliError::Store(e)
    }
}

#[derive(Parser)]
#[command(name = "taskpond", about = "A small task store CLI", disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: TopCommand,
}

#[derive(Subcommand)]
enum TopCommand {
    /// Work with task items
    Item {
        #[command(subcommand)]
        cmd: ItemCommand,
    },
    /// Work with collections
    Collection {
        #[command(subcommand)]
        cmd: CollectionCommand,
    },
}

#[derive(Subcommand)]
enum ItemCommand {
    Create {
        #[arg(short = 'c', long = "collection")]
        collection: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        title: Vec<String>,
    },
    Get {
        #[arg(short = 's', long = "status")]
        status: Option<String>,
        #[arg(short = 'c', long = "collection")]
        collection: Option<String>,
        ids: Vec<String>,
    },
    Update {
        id: String,
        #[arg(short = 'c', long = "collection")]
        collection: Option<String>,
        #[arg(short = 's', long = "status")]
        status: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        title: Vec<String>,
    },
    Note {
        #[command(subcommand)]
        cmd: NoteCommand,
    },
    Delete {
        #[arg(short = 'c', long = "collection")]
        collection: Option<String>,
        ids: Vec<String>,
    },
}

#[derive(Subcommand)]
enum NoteCommand {
    Add {
        id: String,
        #[arg(long = "body")]
        body: String,
    },
    Update {
        id: String,
        #[arg(long = "body")]
        body: String,
    },
    Delete {
        id: String,
    },
}

#[derive(Subcommand)]
enum CollectionCommand {
    List,
    Create { name: String },
    Rename { old_name: String, new_name: String },
    Color { name: String, color: String },
    Delete { name: String },
    Clear {
        name: String,
        #[arg(long = "completed")]
        completed: bool,
    },
}

/// Parse `args` (including the program name at index 0), run the command against
/// `store`, and write JSON output to `out`. Returns `CliError` on failure.
pub fn run(args: &[String], store: &TaskStore, out: &mut dyn Write) -> Result<(), CliError> {
    let cli = Cli::try_parse_from(args).map_err(CliError::Parse)?;
    match cli.command {
        TopCommand::Item { cmd } => run_item(cmd, store, out),
        TopCommand::Collection { cmd } => run_collection(cmd, store, out),
    }
}

fn run_item(cmd: ItemCommand, _store: &TaskStore, _out: &mut dyn Write) -> Result<(), CliError> {
    match cmd {
        _ => Ok(()), // handlers filled in Tasks 5-7
    }
}

fn run_collection(cmd: CollectionCommand, _store: &TaskStore, _out: &mut dyn Write) -> Result<(), CliError> {
    match cmd {
        _ => Ok(()), // handlers filled in Tasks 8-9
    }
}

#[cfg(test)]
pub(crate) fn run_capture(args: &[&str], store: &TaskStore) -> (Result<(), CliError>, String) {
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut buf: Vec<u8> = Vec::new();
    let res = run(&argv, store, &mut buf);
    (res, String::from_utf8(buf).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn unknown_command_is_a_parse_error() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let (res, _out) = run_capture(&["taskpond", "bogus"], &store);
        assert!(matches!(res, Err(CliError::Parse(_))));
    }
}
```

Note: the stub `match cmd { _ => Ok(()) }` arms will be replaced by real arms in later tasks; the `unused` imports (`ItemOutput`, etc.) are used once handlers land — if clippy flags unused imports in this intermediate task, add a temporary `#![allow(unused_imports)]` at the top of `cli.rs` and REMOVE it in Task 9. Report doing so.

- [ ] **Step 2: Wire the module + re-export** — set `crates/taskpond-cli/src/lib.rs` to:

```rust
//! The `taskpond` CLI: a thin command layer over `pond-core`.

pub mod cli;
pub mod output;
pub mod parse;

pub use cli::{run, CliError};
```

- [ ] **Step 3: Set `crates/taskpond-cli/src/main.rs`**

```rust
use pond_core::TaskStore;
use std::io::{self, Write};
use taskpond_cli::{run, CliError};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let store = TaskStore::open_default();
    let mut stdout = io::stdout();
    match run(&args, &store, &mut stdout) {
        Ok(()) => {
            let _ = stdout.flush();
        }
        Err(CliError::Parse(e)) => {
            // clap prints help/usage itself and chooses the exit code (0 for --help, 2 for misuse).
            e.exit();
        }
        Err(CliError::Store(e)) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
        Err(CliError::Usage(message)) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p taskpond-cli cli`
Expected: PASS (`unknown_command_is_a_parse_error`).

- [ ] **Step 5: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p taskpond-cli -- -D warnings
git add crates/taskpond-cli
git commit -m "feat(cli): add command tree and run() entry point"
```

---

## Task 5: `item create` + `item get`

**Files:**
- Modify: `crates/taskpond-cli/src/cli.rs`

Replace the `run_item` stub with real `Create` and `Get` arms (other item arms still fall through to `Ok(())` until Tasks 6–7). `create` defaults to `Ready` and the default collection; `get` enforces the collection-XOR-ids rule.

- [ ] **Step 1: Write the failing test** — replace `run_item` in `cli.rs` with:

```rust
fn run_item(cmd: ItemCommand, store: &TaskStore, out: &mut dyn Write) -> Result<(), CliError> {
    match cmd {
        ItemCommand::Create { collection, title } => {
            // Swift fires "Create requires a title." only when no title tokens are given;
            // a whitespace-only title falls through to store.add, which returns InvalidTitle.
            if title.is_empty() {
                return Err(CliError::Usage("Create requires a title.".to_string()));
            }
            let title = unescape(&title.join(" "));
            let collection = collection.unwrap_or_else(|| DEFAULT_COLLECTION.to_string());
            let item = store.add(&title, &collection, None, false, pond_core::TaskStatus::Ready)?;
            print_items(out, &[item])
        }
        ItemCommand::Get { status, collection, ids } => {
            if collection.is_some() && !ids.is_empty() {
                return Err(CliError::Store(StoreError::TargetConflict));
            }
            let status = match status {
                Some(s) => Some(parse_status(&s).map_err(CliError::Usage)?),
                None => None,
            };
            let items = store.items(status, collection.as_deref(), &ids, None)?;
            print_items(out, &items)
        }
        _ => Ok(()),
    }
}

fn print_items(out: &mut dyn Write, items: &[pond_core::TaskItem]) -> Result<(), CliError> {
    let outputs: Vec<ItemOutput> = items.iter().map(ItemOutput::from_item).collect();
    let json = to_pretty_sorted(&outputs).map_err(CliError::Store)?;
    writeln!(out, "{json}").map_err(|e| CliError::Usage(e.to_string()))
}
```

Add these tests to the `cli.rs` `mod tests`:

```rust
    #[test]
    fn create_then_get_round_trips_json() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let (res, out) = run_capture(&["taskpond", "item", "create", "-c", "Inbox", "Buy", "milk"], &store);
        assert!(res.is_ok());
        assert!(out.contains("\"title\": \"Buy milk\""));
        assert!(out.contains("\"status\": \"ready\""));
        assert!(out.contains("\"collection\": \"Inbox\""));

        let (res, out) = run_capture(&["taskpond", "item", "get"], &store);
        assert!(res.is_ok());
        assert!(out.contains("Buy milk"));
    }

    #[test]
    fn create_requires_a_title() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let (res, _out) = run_capture(&["taskpond", "item", "create", "-c", "Inbox"], &store);
        assert!(matches!(res, Err(CliError::Usage(m)) if m == "Create requires a title."));
    }

    #[test]
    fn create_whitespace_title_is_invalid() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        // A whitespace-only title is not "no title" — it reaches the store, which
        // rejects it with InvalidTitle (matching Swift).
        let (res, _out) = run_capture(&["taskpond", "item", "create", "-c", "Inbox", "   "], &store);
        assert!(matches!(res, Err(CliError::Store(StoreError::InvalidTitle))));
    }

    #[test]
    fn get_rejects_collection_and_ids_together() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("00000001"), false, pond_core::TaskStatus::Ready).unwrap();
        let (res, _out) = run_capture(&["taskpond", "item", "get", "-c", "Inbox", "00000001"], &store);
        assert!(matches!(res, Err(CliError::Store(StoreError::TargetConflict))));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p taskpond-cli cli`
Expected: PASS (prior + 3 new).

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p taskpond-cli -- -D warnings
git add crates/taskpond-cli/src/cli.rs
git commit -m "feat(cli): implement item create and get"
```

---

## Task 6: `item update` + `item delete`

**Files:**
- Modify: `crates/taskpond-cli/src/cli.rs`

`update` requires at least one of title/collection/status (`MissingUpdate`); `delete` requires a target and forbids both (`pond-core`'s `delete_many` enforces this).

- [ ] **Step 1: Write the failing test** — add these arms to the `match cmd` in `run_item` (replace the `_ => Ok(())` with the two arms followed by a final `ItemCommand::Note { .. } => Ok(())` placeholder kept until Task 7):

```rust
        ItemCommand::Update { id, collection, status, title } => {
            let title = if title.is_empty() { None } else { Some(unescape(&title.join(" "))) };
            let status = match status {
                Some(s) => Some(parse_status(&s).map_err(CliError::Usage)?),
                None => None,
            };
            let item = store.update(&id, title.as_deref(), collection.as_deref(), status)?;
            print_items(out, &[item])
        }
        ItemCommand::Delete { collection, ids } => {
            let deleted = store.delete_many(&ids, collection.as_deref())?;
            print_items(out, &deleted)
        }
        ItemCommand::Note { .. } => Ok(()), // filled in Task 7
```

Add tests:

```rust
    #[test]
    fn update_changes_fields() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("old", "Inbox", Some("0123abcd"), false, pond_core::TaskStatus::Ready).unwrap();
        let (res, out) = run_capture(&["taskpond", "item", "update", "0123abcd", "-s", "in-progress", "new", "title"], &store);
        assert!(res.is_ok());
        assert!(out.contains("\"title\": \"new title\""));
        assert!(out.contains("\"status\": \"in-progress\""));
    }

    #[test]
    fn update_requires_a_field() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("0123abcd"), false, pond_core::TaskStatus::Ready).unwrap();
        let (res, _out) = run_capture(&["taskpond", "item", "update", "0123abcd"], &store);
        assert!(matches!(res, Err(CliError::Store(StoreError::MissingUpdate))));
    }

    #[test]
    fn delete_requires_a_target() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let (res, _out) = run_capture(&["taskpond", "item", "delete"], &store);
        assert!(matches!(res, Err(CliError::Store(StoreError::MissingTarget))));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p taskpond-cli cli`
Expected: PASS.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p taskpond-cli -- -D warnings
git add crates/taskpond-cli/src/cli.rs
git commit -m "feat(cli): implement item update and delete"
```

---

## Task 7: `item note add/update/delete`

**Files:**
- Modify: `crates/taskpond-cli/src/cli.rs`

- [ ] **Step 1: Write the failing test** — replace the `ItemCommand::Note { .. } => Ok(())` placeholder with:

```rust
        ItemCommand::Note { cmd } => match cmd {
            NoteCommand::Add { id, body } => {
                let item = store.add_note(&id, &unescape(&body))?;
                print_items(out, &[item])
            }
            NoteCommand::Update { id, body } => {
                let item = store.update_note(&id, &unescape(&body))?;
                print_items(out, &[item])
            }
            NoteCommand::Delete { id } => {
                let item = store.delete_note(&id)?;
                print_items(out, &[item])
            }
        },
```

Add tests:

```rust
    #[test]
    fn note_add_update_delete() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("0123abcd"), false, pond_core::TaskStatus::Ready).unwrap();

        let (res, out) = run_capture(&["taskpond", "item", "note", "add", "0123abcd", "--body", "hello"], &store);
        assert!(res.is_ok());
        assert!(out.contains("\"body\": \"hello\""));

        let (res, out) = run_capture(&["taskpond", "item", "note", "update", "0123abcd", "--body", "world"], &store);
        assert!(res.is_ok());
        assert!(out.contains("\"body\": \"world\""));

        let (res, out) = run_capture(&["taskpond", "item", "note", "delete", "0123abcd"], &store);
        assert!(res.is_ok());
        assert!(!out.contains("\"note\""));
    }

    #[test]
    fn note_add_to_missing_item_errors() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let (res, _out) = run_capture(&["taskpond", "item", "note", "add", "deadbeef", "--body", "x"], &store);
        assert!(matches!(res, Err(CliError::Store(StoreError::NotFound(_)))));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p taskpond-cli cli`
Expected: PASS.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p taskpond-cli -- -D warnings
git add crates/taskpond-cli/src/cli.rs
git commit -m "feat(cli): implement item note commands"
```

---

## Task 8: `collection list/create/rename/color`

**Files:**
- Modify: `crates/taskpond-cli/src/cli.rs`

Replace the `run_collection` stub. `create`/`rename` print the resulting collection's summary (looked up via `collection_summaries`); `color` uses the summary `set_collection_color` returns directly.

- [ ] **Step 1: Write the failing test** — set `run_collection` to:

```rust
fn run_collection(cmd: CollectionCommand, store: &TaskStore, out: &mut dyn Write) -> Result<(), CliError> {
    match cmd {
        CollectionCommand::List => {
            let summaries = store.collection_summaries()?;
            print_collections(out, &summaries)
        }
        CollectionCommand::Create { name } => {
            let created = store.create_collection(&unescape(&name), DEFAULT_GROUP)?;
            print_collection_named(out, store, &created)
        }
        CollectionCommand::Rename { old_name, new_name } => {
            let final_name = store.rename_collection(&unescape(&old_name), &unescape(&new_name))?;
            print_collection_named(out, store, &final_name)
        }
        CollectionCommand::Color { name, color } => {
            let color = parse_color(&color).map_err(CliError::Usage)?;
            let summary = store.set_collection_color(&unescape(&name), color)?;
            print_collections(out, &[summary])
        }
        _ => Ok(()), // delete/clear filled in Task 9
    }
}

fn print_collections(out: &mut dyn Write, summaries: &[pond_core::CollectionSummary]) -> Result<(), CliError> {
    let outputs: Vec<CollectionOutput> = summaries.iter().map(CollectionOutput::from_summary).collect();
    let json = to_pretty_sorted(&outputs).map_err(CliError::Store)?;
    writeln!(out, "{json}").map_err(|e| CliError::Usage(e.to_string()))
}

/// Look up a collection summary by api name and print it (matches Swift's printCollection).
fn print_collection_named(out: &mut dyn Write, store: &TaskStore, name: &str) -> Result<(), CliError> {
    let summary = store
        .collection_summaries()?
        .into_iter()
        .find(|c| c.name == name)
        .ok_or_else(|| CliError::Store(StoreError::CollectionNotFound(name.to_string())))?;
    print_collections(out, &[summary])
}
```

Add tests:

```rust
    #[test]
    fn collection_create_list_rename_color() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));

        let (res, out) = run_capture(&["taskpond", "collection", "create", "Errands"], &store);
        assert!(res.is_ok());
        assert!(out.contains("\"name\": \"Errands\""));

        let (res, out) = run_capture(&["taskpond", "collection", "color", "Errands", "blue"], &store);
        assert!(res.is_ok());
        assert!(out.contains("\"color\": \"blue\""));

        let (res, out) = run_capture(&["taskpond", "collection", "rename", "Errands", "Personal"], &store);
        assert!(res.is_ok());
        assert!(out.contains("\"name\": \"Personal\""));

        let (res, out) = run_capture(&["taskpond", "collection", "list"], &store);
        assert!(res.is_ok());
        assert!(out.contains("Personal"));
    }

    #[test]
    fn collection_color_rejects_bad_color() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.create_collection("Errands", DEFAULT_GROUP).unwrap();
        let (res, _out) = run_capture(&["taskpond", "collection", "color", "Errands", "teal"], &store);
        assert!(matches!(res, Err(CliError::Usage(m)) if m.contains("Expected 'gray'")));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p taskpond-cli cli`
Expected: PASS.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p taskpond-cli -- -D warnings
git add crates/taskpond-cli/src/cli.rs
git commit -m "feat(cli): implement collection list/create/rename/color"
```

---

## Task 9: `collection delete/clear`

**Files:**
- Modify: `crates/taskpond-cli/src/cli.rs`

`delete` prints the collection's summary captured *before* deletion (matches Swift). If a `#![allow(unused_imports)]` was added in Task 4, remove it now and confirm clippy is clean.

- [ ] **Step 1: Write the failing test** — replace the `_ => Ok(())` arm in `run_collection` with:

```rust
        CollectionCommand::Delete { name } => {
            let clean = unescape(&name);
            let summary = store
                .collection_summaries()?
                .into_iter()
                .find(|c| c.name == clean)
                .ok_or_else(|| CliError::Store(StoreError::CollectionNotFound(clean.clone())))?;
            store.delete_collection(&clean)?;
            print_collections(out, &[summary])
        }
        CollectionCommand::Clear { name, completed } => {
            let cleared = store.clear_items(&unescape(&name), completed)?;
            print_items(out, &cleared)
        }
```

Add tests:

```rust
    #[test]
    fn collection_delete_prints_predelete_summary() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Work/A", Some("00000001"), false, pond_core::TaskStatus::Ready).unwrap();
        let (res, out) = run_capture(&["taskpond", "collection", "delete", "Work/A"], &store);
        assert!(res.is_ok());
        assert!(out.contains("\"name\": \"Work/A\""));
        assert!(out.contains("\"totalCount\": 1"));
        // collection is gone afterward
        let (_res, out) = run_capture(&["taskpond", "collection", "list"], &store);
        assert!(!out.contains("Work/A"));
    }

    #[test]
    fn collection_clear_completed_only() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("done", "Inbox", Some("00000001"), false, pond_core::TaskStatus::Completed).unwrap();
        store.add("open", "Inbox", Some("00000002"), false, pond_core::TaskStatus::Ready).unwrap();
        let (res, out) = run_capture(&["taskpond", "collection", "clear", "Inbox", "--completed"], &store);
        assert!(res.is_ok());
        assert!(out.contains("done"));
        assert!(store.items(None, Some("Inbox"), &[], None).unwrap().len() == 1);
    }
```

- [ ] **Step 2: Run tests + remove any temporary allow**

Run: `cargo test -p taskpond-cli`
Expected: PASS (whole CLI crate). Remove the temporary `#![allow(unused_imports)]` from Task 4 if present.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p taskpond-cli -- -D warnings
git add crates/taskpond-cli/src/cli.rs
git commit -m "feat(cli): implement collection delete/clear"
```

---

## Task 10: Installer types + `status()` (`pond-core::cli_install`)

**Files:**
- Create: `crates/pond-core/src/cli_install.rs`
- Modify: `crates/pond-core/src/lib.rs`

Port the installer's types and `status()` / link classification. `cfg(unix)`. Uses `std::os::unix::fs::symlink`, `fs::symlink_metadata` (does not follow symlinks), `fs::read_link`.

- [ ] **Step 1: Write the failing test** — create `crates/pond-core/src/cli_install.rs`

```rust
//! Command-line installer: symlinks `~/.local/bin/taskpond` at the bundled binary.
//! Unix-only (macOS-primary); port of the Swift `CommandLineInstaller`.
#![cfg(unix)]

use crate::error::{Result, StoreError};
use crate::paths::data_directory;
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallStatus {
    pub link_path: PathBuf,
    pub target_path: PathBuf,
    pub installed: bool,
    pub conflict_description: Option<String>,
    pub install_directory_is_in_path: bool,
    pub can_uninstall: bool,
}

impl InstallStatus {
    pub fn can_install(&self) -> bool {
        !self.installed && (self.conflict_description.is_none() || self.can_uninstall)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallRecord {
    link_path: String,
    target_path: String,
    installed_at: chrono::DateTime<chrono::Utc>,
}

pub struct Installer {
    pub link_path: PathBuf,
    pub target_path: PathBuf,
    pub record_path: PathBuf,
}

enum LinkKind {
    Missing,
    File,
    Symlink(PathBuf),
}

fn link_kind(path: &Path) -> LinkKind {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return LinkKind::Missing,
    };
    if meta.file_type().is_symlink() {
        match fs::read_link(path) {
            Ok(dest) => {
                let resolved = if dest.is_absolute() {
                    dest
                } else {
                    path.parent().unwrap_or(Path::new("")).join(dest)
                };
                LinkKind::Symlink(resolved)
            }
            Err(_) => LinkKind::File,
        }
    } else {
        LinkKind::File
    }
}

fn path_contains(dir: &Path) -> bool {
    let target = dir.to_string_lossy();
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .any(|entry| entry == target)
}

impl Installer {
    /// Construct with explicit paths (used by tests and the GUI).
    pub fn new(link_path: PathBuf, target_path: PathBuf, record_path: PathBuf) -> Self {
        Installer { link_path, target_path, record_path }
    }

    /// Default link (`~/.local/bin/taskpond`) and record (`<data-dir>/cli-install.json`).
    /// The target defaults to the current executable; packaging finalizes this for the
    /// Tauri bundle in a later phase.
    pub fn with_defaults() -> Self {
        let home = directories::BaseDirs::new()
            .map(|b| b.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let target = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("taskpond"));
        Installer {
            link_path: home.join(".local/bin/taskpond"),
            target_path: target,
            record_path: data_directory().join("cli-install.json"),
        }
    }

    pub fn path_hint(&self) -> String {
        r#"export PATH="$HOME/.local/bin:$PATH""#.to_string()
    }

    pub fn status(&self) -> InstallStatus {
        let in_path = self
            .link_path
            .parent()
            .map(path_contains)
            .unwrap_or(false);
        match link_kind(&self.link_path) {
            LinkKind::Missing => InstallStatus {
                link_path: self.link_path.clone(),
                target_path: self.target_path.clone(),
                installed: false,
                conflict_description: None,
                install_directory_is_in_path: in_path,
                can_uninstall: false,
            },
            LinkKind::File => InstallStatus {
                link_path: self.link_path.clone(),
                target_path: self.target_path.clone(),
                installed: false,
                conflict_description: Some(format!("{} already exists.", self.link_path.display())),
                install_directory_is_in_path: in_path,
                can_uninstall: false,
            },
            LinkKind::Symlink(dest) => {
                let installed = dest == self.target_path;
                InstallStatus {
                    link_path: self.link_path.clone(),
                    target_path: self.target_path.clone(),
                    installed,
                    conflict_description: if installed {
                        None
                    } else {
                        Some(format!("{} points to {}.", self.link_path.display(), dest.display()))
                    },
                    install_directory_is_in_path: in_path,
                    can_uninstall: self.can_remove_symlink(&dest),
                }
            }
        }
    }

    fn can_remove_symlink(&self, dest: &Path) -> bool {
        if dest == self.target_path {
            return true;
        }
        match self.read_record() {
            Some(record) => Path::new(&record.target_path) == dest,
            None => false,
        }
    }

    fn read_record(&self) -> Option<InstallRecord> {
        let data = fs::read(&self.record_path).ok()?;
        serde_json::from_slice(&data).ok()
    }

    pub(crate) fn is_executable(path: &Path) -> bool {
        fs::metadata(path)
            .map(|m| m.is_file() && (m.permissions().mode() & 0o111 != 0))
            .unwrap_or(false)
    }

    pub(crate) fn _unused(&self) {
        let _ = StoreError::InvalidTitle; // keep StoreError import until Task 11 uses it
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    fn make_executable(path: &Path) {
        fs::write(path, b"#!/bin/sh\n").unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn status_missing_then_installed() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("taskpond-bin");
        make_executable(&target);
        let link = dir.path().join("link");
        let record = dir.path().join("cli-install.json");
        let installer = Installer::new(link.clone(), target.clone(), record);

        let status = installer.status();
        assert!(!status.installed);
        assert!(status.conflict_description.is_none());

        symlink(&target, &link).unwrap();
        let status = installer.status();
        assert!(status.installed);
        assert!(status.can_install() == false);
    }

    #[test]
    fn status_conflict_for_foreign_symlink() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("taskpond-bin");
        make_executable(&target);
        let other = dir.path().join("other-bin");
        make_executable(&other);
        let link = dir.path().join("link");
        symlink(&other, &link).unwrap();
        let installer = Installer::new(link, target, dir.path().join("rec.json"));
        let status = installer.status();
        assert!(!status.installed);
        assert!(status.conflict_description.as_ref().unwrap().contains("points to"));
        assert!(!status.can_uninstall); // foreign symlink, no record
    }
}
```

Note: the `_unused` shim and the `StoreError` import exist only so this task compiles clean under `-D warnings` before Task 11 adds `install`/`uninstall` (which use `StoreError`). Task 11 removes `_unused`.

- [ ] **Step 2: Wire the module** — append to `crates/pond-core/src/lib.rs`:

```rust
#[cfg(unix)]
pub mod cli_install;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p pond-core cli_install`
Expected: PASS (2 tests).

- [ ] **Step 4: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/cli_install.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add cli_install status + types"
```

---

## Task 11: Installer `install()` / `uninstall()` + record

**Files:**
- Modify: `crates/pond-core/src/cli_install.rs`

Port `install`, `uninstall`, and record read/write. Replace the `_unused` shim. Errors use `StoreError` (a new `Io`/message via existing variants): use `StoreError::Io(String)` for filesystem failures and dedicated messages for the conflict cases.

- [ ] **Step 1: Write the failing test** — remove the `_unused` method and add to `impl Installer`:

```rust
    pub fn install(&self) -> Result<()> {
        if !Self::is_executable(&self.target_path) {
            return Err(StoreError::Io(format!(
                "CLI executable was not found at {}.",
                self.target_path.display()
            )));
        }
        if let Some(parent) = self.link_path.parent() {
            fs::create_dir_all(parent)?;
        }
        match link_kind(&self.link_path) {
            LinkKind::Missing => {
                std::os::unix::fs::symlink(&self.target_path, &self.link_path)?;
                self.write_record()
            }
            LinkKind::File => Err(StoreError::Io(format!(
                "{} already exists and is not a symlink created by Pond.",
                self.link_path.display()
            ))),
            LinkKind::Symlink(dest) => {
                if dest == self.target_path {
                    self.write_record()
                } else if self.can_remove_symlink(&dest) {
                    fs::remove_file(&self.link_path)?;
                    std::os::unix::fs::symlink(&self.target_path, &self.link_path)?;
                    self.write_record()
                } else {
                    Err(StoreError::Io(format!(
                        "{} already points to {}.",
                        self.link_path.display(),
                        dest.display()
                    )))
                }
            }
        }
    }

    pub fn uninstall(&self) -> Result<()> {
        match link_kind(&self.link_path) {
            LinkKind::Missing => {
                let _ = fs::remove_file(&self.record_path);
                Ok(())
            }
            LinkKind::File => Err(StoreError::Io(format!(
                "{} already exists and is not a symlink created by Pond.",
                self.link_path.display()
            ))),
            LinkKind::Symlink(dest) => {
                if self.can_remove_symlink(&dest) {
                    fs::remove_file(&self.link_path)?;
                    let _ = fs::remove_file(&self.record_path);
                    Ok(())
                } else {
                    Err(StoreError::Io(format!(
                        "{} already points to {}.",
                        self.link_path.display(),
                        dest.display()
                    )))
                }
            }
        }
    }

    fn write_record(&self) -> Result<()> {
        if let Some(parent) = self.record_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let record = InstallRecord {
            link_path: self.link_path.to_string_lossy().to_string(),
            target_path: self.target_path.to_string_lossy().to_string(),
            installed_at: chrono::Utc::now(),
        };
        let json = crate::json::to_pretty_sorted(&record)?;
        fs::write(&self.record_path, json.as_bytes())?;
        Ok(())
    }
```

Add tests to `cli_install`'s `mod tests`:

```rust
    #[test]
    fn install_then_uninstall_round_trip() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("taskpond-bin");
        make_executable(&target);
        let link = dir.path().join("bin/taskpond");
        let record = dir.path().join("cli-install.json");
        let installer = Installer::new(link.clone(), target.clone(), record.clone());

        installer.install().unwrap();
        assert!(installer.status().installed);
        assert!(record.exists());

        installer.uninstall().unwrap();
        assert!(!installer.status().installed);
        assert!(!record.exists());
        assert!(fs::symlink_metadata(&link).is_err()); // link removed
    }

    #[test]
    fn install_rejects_missing_executable() {
        let dir = tempdir().unwrap();
        let installer = Installer::new(
            dir.path().join("link"),
            dir.path().join("does-not-exist"),
            dir.path().join("rec.json"),
        );
        let err = installer.install().unwrap_err();
        assert!(format!("{err}").contains("was not found"));
    }

    #[test]
    fn install_refuses_to_clobber_foreign_file() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("taskpond-bin");
        make_executable(&target);
        let link = dir.path().join("link");
        fs::write(&link, b"i am a real file").unwrap();
        let installer = Installer::new(link, target, dir.path().join("rec.json"));
        let err = installer.install().unwrap_err();
        assert!(format!("{err}").contains("not a symlink created by Pond"));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p pond-core cli_install`
Expected: PASS (prior 2 + new 3).

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/cli_install.rs
git commit -m "feat(core): add cli_install install/uninstall + record"
```

---

## Task 12: Workspace gate + end-to-end CLI↔store test + README

**Files:**
- Create: `crates/taskpond-cli/tests/cli_store_roundtrip.rs`
- Modify: `README.md`

Confirm the whole workspace is green and that the CLI and store agree on the on-disk file via `POND_STORE`.

- [ ] **Step 1: Write the failing integration test** — create `crates/taskpond-cli/tests/cli_store_roundtrip.rs`

```rust
use pond_core::TaskStore;
use taskpond_cli::run;

// Two CLI invocations against the same store file must see each other's writes.
#[test]
fn cli_invocations_share_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = TaskStore::new(dir.path().join("tasks.json"));

    let create: Vec<String> = ["taskpond", "item", "create", "-c", "Inbox", "Ship", "it"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut sink: Vec<u8> = Vec::new();
    run(&create, &store, &mut sink).unwrap();

    // A fresh TaskStore on the same path sees the item.
    let store2 = TaskStore::new(dir.path().join("tasks.json"));
    let get: Vec<String> = ["taskpond", "item", "get"].iter().map(|s| s.to_string()).collect();
    let mut out: Vec<u8> = Vec::new();
    run(&get, &store2, &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("Ship it"));
}
```

- [ ] **Step 2: Run the whole workspace**

Run: `cargo test` (from repo root)
Expected: PASS — `pond-core` (66 + 5 installer = 71) and `taskpond-cli` (unit + the integration test) all green.

- [ ] **Step 3: Lint + format the workspace**

Run: `cargo fmt && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 4: Update `README.md` build/CLI note** — under the build instructions, add a short Rust note (place after the existing CLI section):

```markdown
## Rust workspace (migration in progress)

The Tauri rewrite lives in `crates/`:

- `pond-core` — the data store + domain logic (also hosts the macOS `cli_install` module).
- `taskpond-cli` — the `taskpond` CLI binary.

Build/test: `cargo test`. Run the CLI: `cargo run -p taskpond-cli -- item get`.
The store path honors `POND_STORE`.
```

- [ ] **Step 5: Commit**

```bash
git add crates/taskpond-cli/tests/cli_store_roundtrip.rs README.md
git commit -m "test(cli): add CLI/store round-trip + workspace gate; document Rust crates"
```

---

## Self-Review (completed during planning)

**Spec coverage (Phase 2 design doc):**
- `taskpond-cli` crate (lib `run()` + `taskpond` bin) → Tasks 1, 4, 12. ✅
- Command surface → `pond-core` mapping (item create/get/update/note/delete; collection list/create/rename/color/delete/clear) → Tasks 5–9. ✅
- stdout JSON shapes (ItemOutput/CollectionOutput, rawValue enums, omit absent note/indicator, sorted, no dates) → Task 3. ✅
- `\n`/`\t` unescaping, status/color parsing, XOR-target rule → Tasks 2, 5, 6. ✅
- Errors: `StoreError` parity to stderr + exit 1; clap for structural parse errors; bespoke "Create requires a title" → Tasks 4, 5. ✅
- Installer `pond-core::cli_install` (`cfg(unix)`): types, status, install/uninstall, record, defaults, injectable paths → Tasks 10, 11. ✅
- Testing: in-process `run()` CLI tests + injected-path installer tests + a store round-trip → Tasks 5–12. ✅
- Deferred production target (current_exe default) → Task 10 (`with_defaults`). ✅

**Placeholder scan:** No TBD/"handle errors" placeholders; every code step is complete. The two intentional temporary shims (`#![allow(unused_imports)]` in Task 4, `_unused`/`StoreError` keep-alive in Task 10) are explicitly introduced *and removed* in Tasks 9 and 11 respectively, with the removal called out. ✅

**Type consistency:** `run(args: &[String], store: &TaskStore, out: &mut dyn Write) -> Result<(), CliError>` is used identically in `main.rs`, `run_capture`, and the integration test. `CliError` variants (`Parse`/`Store`/`Usage`) are consistent. Handlers call the real `pond-core` signatures verified in Phase 1 (`add`, `items`, `update`, `add_note`/`update_note`/`delete_note`, `delete_many`, `collection_summaries`, `create_collection`, `rename_collection`, `set_collection_color`, `delete_collection`, `clear_items`). `to_pretty_sorted`, `DEFAULT_GROUP`, `DEFAULT_COLLECTION` are the real `pond-core` public names. `StoreError::Io(String)` exists (Phase 1 error enum). ✅

**Note on `StoreError::Io` for installer errors:** the Swift installer had dedicated error cases; this plan reuses `pond-core`'s existing `StoreError::Io(String)` with parity message text rather than adding new variants, keeping the error surface small. If a future phase wants typed installer errors, that's a clean follow-up.
