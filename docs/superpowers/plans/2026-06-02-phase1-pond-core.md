# Phase 1: `pond-core` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `pond-core`, a pure Rust library that ports the Swift `TaskCore` (domain model + file-backed JSON store) with full unit-test coverage, ready to be shared by the Tauri app and the `taskpond` CLI.

**Architecture:** A `cargo` workspace with a `pond-core` library crate. The crate has no UI/Tauri dependency: domain types, a file-locked + atomic JSON store, collection/group normalization, item operations (including merge/split), prompt-template evaluation, and export encoding. The Swift sources under `Sources/TaskCore/` are the behavioral source of truth.

**Tech Stack:** Rust (2021 edition), `serde` + `serde_json` (JSON; sorted keys via `to_value`), `chrono` (RFC3339 timestamps), `directories` (cross-platform paths), `fs2` (advisory file lock), `rand` (id/version generation), `thiserror` (errors), `tempfile` (dev/tests).

**Conventions for every task below:**
- Run `cargo test -p pond-core` from the repo root.
- The store is constructed with an explicit path in tests (`TaskStore::new(path)`); never rely on the global default path inside a test, so tests stay isolated and parallel-safe.
- After a task's tests pass, run `cargo fmt` and `cargo clippy -p pond-core -- -D warnings` before committing.

---

## File Structure

```
crates/pond-core/
├─ Cargo.toml
└─ src/
   ├─ lib.rs          # module wiring + public re-exports
   ├─ error.rs        # StoreError
   ├─ ids.rs          # make_id, make_version, is_valid_id
   ├─ model.rs        # TaskStatus, CollectionColor, TaskNote, TaskItem, summaries
   ├─ json.rs         # sorted-key JSON encoder helpers
   ├─ document.rs     # TaskFile (on-disk document) + TaskCollectionGroup
   ├─ paths.rs        # default store path (+ POND_STORE override)
   ├─ collections.rs  # name/group normalization, summaries builders
   ├─ prompt.rs       # PromptTemplate evaluation
   ├─ export.rs       # collection export (JSON / JSONL)
   └─ store.rs        # TaskStore: with_file + all operations
Cargo.toml            # workspace root
```

Each module has one responsibility. `store.rs` is the only stateful type; everything else is data + pure functions. Tests live in `#[cfg(test)] mod tests` at the bottom of each module (Rust convention), except store tests which may grow into `crates/pond-core/tests/store.rs` integration tests.

---

## Task 1: Workspace + crate scaffold

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/pond-core/Cargo.toml`
- Create: `crates/pond-core/src/lib.rs`

- [ ] **Step 1: Create the workspace root `Cargo.toml`**

```toml
[workspace]
members = ["crates/pond-core"]
resolver = "2"
```

- [ ] **Step 2: Create `crates/pond-core/Cargo.toml`**

```toml
[package]
name = "pond-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
directories = "5"
fs2 = "0.4"
rand = "0.8"
thiserror = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Create `crates/pond-core/src/lib.rs` with a smoke test**

```rust
//! Pond core: domain model and file-backed task store.

#[cfg(test)]
mod smoke {
    #[test]
    fn workspace_builds() {
        assert_eq!(2 + 2, 4);
    }
}
```

- [ ] **Step 4: Verify it builds and tests pass**

Run: `cargo test -p pond-core`
Expected: PASS (1 test `smoke::workspace_builds`).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/pond-core/Cargo.toml crates/pond-core/src/lib.rs
git commit -m "feat(core): scaffold pond-core crate in a cargo workspace"
```

---

## Task 2: Error type

**Files:**
- Create: `crates/pond-core/src/error.rs`
- Modify: `crates/pond-core/src/lib.rs`

Mirrors `Sources/TaskCore/TaskStoreError.swift`. `PartialEq` lets tests assert on specific variants.

- [ ] **Step 1: Write the failing test** — create `crates/pond-core/src/error.rs`

```rust
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    InvalidTitle,
    InvalidCollection,
    InvalidCollectionGroup,
    DefaultCollection,
    DefaultCollectionGroup,
    InvalidId(String),
    MissingTarget,
    MissingUpdate,
    MissingNoteUpdate,
    TargetConflict,
    NoMatchingTasks,
    NotFound(String),
    NoteNotFound(String),
    CollectionNotFound(String),
    CollectionGroupNotFound(String),
    CollectionConflict(String),
    AmbiguousId(String, Vec<String>),
    DuplicateId(String),
    InvalidNote,
    FileLockFailed(String),
    Io(String),
    Serde(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::InvalidTitle => write!(f, "Task title cannot be empty."),
            StoreError::InvalidCollection => write!(f, "Collection name cannot be empty."),
            StoreError::InvalidCollectionGroup => write!(f, "Collection group name cannot be empty."),
            StoreError::DefaultCollection => write!(f, "Default collection cannot be renamed, deleted, or moved."),
            StoreError::DefaultCollectionGroup => write!(f, "Default collection group cannot be renamed or deleted."),
            StoreError::InvalidId(id) => write!(f, "Task id '{id}' is invalid."),
            StoreError::MissingTarget => write!(f, "Command requires --collection or at least one id."),
            StoreError::MissingUpdate => write!(f, "Update requires a title, --collection, or --status/-s."),
            StoreError::MissingNoteUpdate => write!(f, "Note update requires --body."),
            StoreError::TargetConflict => write!(f, "Use either --collection or ids, not both."),
            StoreError::NoMatchingTasks => write!(f, "No matching tasks."),
            StoreError::NotFound(id) => write!(f, "No task matches '{id}'."),
            StoreError::NoteNotFound(id) => write!(f, "No note matches '{id}'."),
            StoreError::CollectionNotFound(name) => write!(f, "No collection matches '{name}'."),
            StoreError::CollectionGroupNotFound(name) => write!(f, "No collection group matches '{name}'."),
            StoreError::CollectionConflict(name) => write!(f, "Collection '{name}' already exists."),
            StoreError::AmbiguousId(id, matches) => {
                write!(f, "Task id '{id}' is ambiguous: {}.", matches.join(", "))
            }
            StoreError::DuplicateId(id) => write!(f, "Task id '{id}' already exists."),
            StoreError::InvalidNote => write!(f, "Note body cannot be empty."),
            StoreError::FileLockFailed(reason) => write!(f, "Could not lock task store: {reason}"),
            StoreError::Io(reason) => write!(f, "{reason}"),
            StoreError::Serde(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(value: std::io::Error) -> Self {
        StoreError::Io(value.to_string())
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(value: serde_json::Error) -> Self {
        StoreError::Serde(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_messages() {
        assert_eq!(StoreError::InvalidTitle.to_string(), "Task title cannot be empty.");
        assert_eq!(StoreError::NotFound("ab".into()).to_string(), "No task matches 'ab'.");
        assert_eq!(
            StoreError::AmbiguousId("a".into(), vec!["a1".into(), "a2".into()]).to_string(),
            "Task id 'a' is ambiguous: a1, a2."
        );
    }
}
```

- [ ] **Step 2: Wire the module** — set `crates/pond-core/src/lib.rs` to:

```rust
//! Pond core: domain model and file-backed task store.

pub mod error;

pub use error::{Result, StoreError};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p pond-core error`
Expected: PASS (`error::tests::renders_messages`).

- [ ] **Step 4: Lint + format**

Run: `cargo fmt && cargo clippy -p pond-core -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/pond-core/src/error.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add StoreError with parity messages"
```

---

## Task 3: Status & color enums

**Files:**
- Create: `crates/pond-core/src/model.rs`
- Modify: `crates/pond-core/src/lib.rs`

Mirrors `TaskStatus` and `TaskCollectionColor` in `TaskItem.swift`. Serde rawValues must match (`in-progress`, `on-hold`). `all()` order matters for UI/bulk parity.

- [ ] **Step 1: Write the failing test** — create `crates/pond-core/src/model.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Draft,
    Ready,
    #[serde(rename = "in-progress")]
    InProgress,
    Completed,
    #[serde(rename = "on-hold")]
    OnHold,
    Rejected,
    Aborted,
}

impl TaskStatus {
    /// Declaration/UI order, matching Swift `TaskStatus.allCases`.
    pub fn all() -> [TaskStatus; 7] {
        [
            TaskStatus::Draft,
            TaskStatus::Ready,
            TaskStatus::InProgress,
            TaskStatus::Completed,
            TaskStatus::OnHold,
            TaskStatus::Rejected,
            TaskStatus::Aborted,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TaskStatus::Draft => "Draft",
            TaskStatus::Ready => "Ready",
            TaskStatus::InProgress => "In Progress",
            TaskStatus::Completed => "Completed",
            TaskStatus::OnHold => "On Hold",
            TaskStatus::Rejected => "Rejected",
            TaskStatus::Aborted => "Aborted",
        }
    }

    pub fn is_incomplete(&self) -> bool {
        *self != TaskStatus::Completed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionColor {
    Gray,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
}

impl Default for CollectionColor {
    fn default() -> Self {
        CollectionColor::Gray
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_serializes_to_raw_values() {
        assert_eq!(serde_json::to_string(&TaskStatus::InProgress).unwrap(), "\"in-progress\"");
        assert_eq!(serde_json::to_string(&TaskStatus::OnHold).unwrap(), "\"on-hold\"");
        assert_eq!(serde_json::to_string(&TaskStatus::Ready).unwrap(), "\"ready\"");
        let parsed: TaskStatus = serde_json::from_str("\"in-progress\"").unwrap();
        assert_eq!(parsed, TaskStatus::InProgress);
    }

    #[test]
    fn incomplete_excludes_completed_only() {
        assert!(TaskStatus::Ready.is_incomplete());
        assert!(TaskStatus::Aborted.is_incomplete());
        assert!(!TaskStatus::Completed.is_incomplete());
    }

    #[test]
    fn color_round_trips() {
        assert_eq!(serde_json::to_string(&CollectionColor::Purple).unwrap(), "\"purple\"");
        let parsed: CollectionColor = serde_json::from_str("\"green\"").unwrap();
        assert_eq!(parsed, CollectionColor::Green);
        assert_eq!(CollectionColor::default(), CollectionColor::Gray);
    }
}
```

- [ ] **Step 2: Wire the module** — append to `crates/pond-core/src/lib.rs`:

```rust
pub mod model;

pub use model::{CollectionColor, TaskStatus};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p pond-core model`
Expected: PASS (3 tests).

- [ ] **Step 4: Lint + format**

Run: `cargo fmt && cargo clippy -p pond-core -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/pond-core/src/model.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add TaskStatus and CollectionColor enums"
```

---

## Task 4: ID & version generation

**Files:**
- Create: `crates/pond-core/src/ids.rs`
- Modify: `crates/pond-core/src/lib.rs`

Mirrors `TaskStore.makeID`, `TaskItem.makeVersion`, and `isValidID` in `TaskItemSupport.swift`. IDs are 8 lowercase hex chars; versions are 12 alphanumeric chars; both avoid a supplied set of existing values.

- [ ] **Step 1: Write the failing test** — create `crates/pond-core/src/ids.rs`

```rust
use rand::Rng;
use std::collections::HashSet;

const ID_CHARS: &[u8] = b"0123456789abcdef";
const VERSION_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn random_string(chars: &[u8], len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| chars[rng.gen_range(0..chars.len())] as char)
        .collect()
}

/// 8-character lowercase hex id, unique against `existing`.
pub fn make_id(existing: &HashSet<String>) -> String {
    loop {
        let id = random_string(ID_CHARS, 8);
        if !existing.contains(&id) {
            return id;
        }
    }
}

/// 12-character alphanumeric version, unique against `existing`.
pub fn make_version(existing: &HashSet<String>) -> String {
    loop {
        let version = random_string(VERSION_CHARS, 12);
        if !existing.contains(&version) {
            return version;
        }
    }
}

pub fn is_valid_id(id: &str) -> bool {
    id.len() == 8 && id.bytes().all(|b| ID_CHARS.contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_valid_and_unique() {
        let mut seen = HashSet::new();
        for _ in 0..200 {
            let id = make_id(&seen);
            assert!(is_valid_id(&id), "{id} should be valid");
            assert!(seen.insert(id));
        }
    }

    #[test]
    fn make_id_avoids_existing() {
        // Exhaust nothing, but force collision avoidance with a near-full small probe.
        let existing: HashSet<String> = ["deadbeef".to_string()].into_iter().collect();
        let id = make_id(&existing);
        assert_ne!(id, "deadbeef");
    }

    #[test]
    fn version_is_twelve_alnum() {
        let v = make_version(&HashSet::new());
        assert_eq!(v.len(), 12);
        assert!(v.bytes().all(|b| b.is_ascii_alphanumeric()));
    }

    #[test]
    fn invalid_ids_rejected() {
        assert!(!is_valid_id("abc"));          // too short
        assert!(!is_valid_id("deadbeeff"));    // too long
        assert!(!is_valid_id("deadbeeg"));     // non-hex
        assert!(is_valid_id("0123abcd"));
    }
}
```

- [ ] **Step 2: Wire the module** — append to `crates/pond-core/src/lib.rs`:

```rust
pub mod ids;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p pond-core ids`
Expected: PASS (4 tests).

- [ ] **Step 4: Lint + format**

Run: `cargo fmt && cargo clippy -p pond-core -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/pond-core/src/ids.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add id and version generation"
```

---

## Task 5: TaskNote & TaskItem

**Files:**
- Modify: `crates/pond-core/src/model.rs`
- Modify: `crates/pond-core/src/lib.rs`

Mirrors `TaskNote` / `TaskItem`. Fresh-start simplification: a task holds `note: Option<TaskNote>` (Swift's array was always 0..1) and the on-disk key is the singular `note`. Timestamps are `DateTime<Utc>` (RFC3339 via serde).

- [ ] **Step 1: Write the failing test** — add to the top of `crates/pond-core/src/model.rs` (after the existing `use`):

```rust
use crate::ids::make_version;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskNote {
    pub id: String,
    pub version: String,
    pub body: String,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskItem {
    pub id: String,
    pub version: String,
    pub title: String,
    pub collection: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<TaskNote>,
    pub status: TaskStatus,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

impl TaskItem {
    pub fn new(id: String, title: String, collection: String, status: TaskStatus, now: DateTime<Utc>) -> Self {
        TaskItem {
            id,
            version: make_version(&HashSet::new()),
            title,
            collection,
            note: None,
            status,
            created_at: now,
            updated_at: now,
        }
    }
}
```

Then add these tests to the `mod tests` block in `model.rs`:

```rust
    #[test]
    fn item_serializes_note_as_singular_and_omits_when_absent() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();
        let mut item = TaskItem::new("0123abcd".into(), "Buy milk".into(), "Inbox".into(), TaskStatus::Ready, now);
        let json = serde_json::to_string(&item).unwrap();
        assert!(!json.contains("\"note\""), "absent note must be omitted: {json}");

        item.note = Some(TaskNote {
            id: "ffff0000".into(),
            version: "abcdefabcdef".into(),
            body: "2%".into(),
            created_at: now,
            updated_at: now,
        });
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"note\""));
        let round: TaskItem = serde_json::from_str(&json).unwrap();
        assert_eq!(round, item);
    }
```

Add `use chrono::TimeZone;` to the `mod tests` block's `use super::*;` line area (i.e. add `use chrono::TimeZone;` beneath `use super::*;`).

- [ ] **Step 2: Wire exports** — replace the model re-export line in `crates/pond-core/src/lib.rs` with:

```rust
pub use model::{CollectionColor, TaskItem, TaskNote, TaskStatus};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p pond-core model`
Expected: PASS (existing 3 + new 1).

- [ ] **Step 4: Lint + format**

Run: `cargo fmt && cargo clippy -p pond-core -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/pond-core/src/model.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add TaskItem and TaskNote types"
```

---

## Task 6: Summary types

**Files:**
- Modify: `crates/pond-core/src/model.rs`
- Modify: `crates/pond-core/src/lib.rs`

Plain data carriers mirroring `TaskCollectionSummary` / `TaskCollectionGroupSummary`. These are returned to callers (CLI/GUI); they are not persisted, so field names are Rust-native.

- [ ] **Step 1: Write the failing test** — add to `crates/pond-core/src/model.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionSummary {
    pub name: String,
    pub display_name: String,
    pub group_name: String,
    pub total_count: usize,
    pub incomplete_count: usize,
    pub status_indicator: Option<TaskStatus>,
    pub color: CollectionColor,
    pub is_archived: bool,
    pub prompt_template: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionGroupSummary {
    pub name: String,
    pub collections: Vec<CollectionSummary>,
}
```

Add to `mod tests`:

```rust
    #[test]
    fn summary_constructs() {
        let summary = CollectionSummary {
            name: "Work/Tasks".into(),
            display_name: "Tasks".into(),
            group_name: "Work".into(),
            total_count: 3,
            incomplete_count: 2,
            status_indicator: Some(TaskStatus::OnHold),
            color: CollectionColor::Blue,
            is_archived: false,
            prompt_template: None,
        };
        assert_eq!(summary.incomplete_count, 2);
        let group = CollectionGroupSummary { name: "Work".into(), collections: vec![summary] };
        assert_eq!(group.collections.len(), 1);
    }
```

- [ ] **Step 2: Wire exports** — replace the model re-export line in `lib.rs` with:

```rust
pub use model::{
    CollectionColor, CollectionGroupSummary, CollectionSummary, TaskItem, TaskNote, TaskStatus,
};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p pond-core model`
Expected: PASS.

- [ ] **Step 4: Lint + format**

Run: `cargo fmt && cargo clippy -p pond-core -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/pond-core/src/model.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add collection summary types"
```

---

## Task 7: JSON encoder helpers

**Files:**
- Create: `crates/pond-core/src/json.rs`
- Modify: `crates/pond-core/src/lib.rs`

Mirrors `PondJSON` (`JSONCoding.swift`): sorted keys, pretty/compact. Sorted keys are achieved by round-tripping through `serde_json::Value`, whose object map is a `BTreeMap` by default (keys iterate sorted).

- [ ] **Step 1: Write the failing test** — create `crates/pond-core/src/json.rs`

```rust
use crate::error::Result;
use serde::Serialize;

/// Pretty-printed JSON with keys sorted alphabetically.
pub fn to_pretty_sorted<T: Serialize>(value: &T) -> Result<String> {
    let value = serde_json::to_value(value)?;
    Ok(serde_json::to_string_pretty(&value)?)
}

/// Compact JSON with keys sorted alphabetically.
pub fn to_compact_sorted<T: Serialize>(value: &T) -> Result<String> {
    let value = serde_json::to_value(value)?;
    Ok(serde_json::to_string(&value)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Sample {
        zebra: i32,
        apple: i32,
    }

    #[test]
    fn keys_are_sorted() {
        let json = to_compact_sorted(&Sample { zebra: 1, apple: 2 }).unwrap();
        assert_eq!(json, r#"{"apple":2,"zebra":1}"#);
    }

    #[test]
    fn pretty_is_multiline_and_sorted() {
        let json = to_pretty_sorted(&Sample { zebra: 1, apple: 2 }).unwrap();
        assert!(json.starts_with("{\n"));
        let apple = json.find("apple").unwrap();
        let zebra = json.find("zebra").unwrap();
        assert!(apple < zebra);
    }
}
```

- [ ] **Step 2: Wire the module** — append to `lib.rs`:

```rust
pub mod json;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p pond-core json`
Expected: PASS (2 tests).

- [ ] **Step 4: Lint + format**

Run: `cargo fmt && cargo clippy -p pond-core -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/pond-core/src/json.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add sorted-key JSON encoder helpers"
```

---

## Task 8: On-disk document (`TaskFile`)

**Files:**
- Create: `crates/pond-core/src/document.rs`
- Modify: `crates/pond-core/src/lib.rs`

The persisted document. Fresh-start schema `version: 1`, camelCase keys. `archived_collections` is a `BTreeSet` (serializes as a sorted array); color/prompt maps are `BTreeMap` (sorted keys). No legacy migration.

- [ ] **Step 1: Write the failing test** — create `crates/pond-core/src/document.rs`

```rust
use crate::model::{CollectionColor, TaskItem};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCollectionGroup {
    pub name: String,
    pub collections: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFile {
    pub version: u32,
    #[serde(default)]
    pub collections: Vec<String>,
    #[serde(default)]
    pub collection_groups: Vec<TaskCollectionGroup>,
    #[serde(default)]
    pub collection_colors: BTreeMap<String, CollectionColor>,
    #[serde(default)]
    pub collection_prompts: BTreeMap<String, String>,
    #[serde(default)]
    pub archived_collections: BTreeSet<String>,
    #[serde(default)]
    pub items: Vec<TaskItem>,
}

impl Default for TaskFile {
    fn default() -> Self {
        TaskFile {
            version: 1,
            collections: Vec::new(),
            collection_groups: Vec::new(),
            collection_colors: BTreeMap::new(),
            collection_prompts: BTreeMap::new(),
            archived_collections: BTreeSet::new(),
            items: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::to_pretty_sorted;

    #[test]
    fn empty_document_round_trips() {
        let file = TaskFile::default();
        let json = to_pretty_sorted(&file).unwrap();
        let parsed: TaskFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, file);
        assert_eq!(parsed.version, 1);
    }

    #[test]
    fn camel_case_keys_used() {
        let json = to_pretty_sorted(&TaskFile::default()).unwrap();
        assert!(json.contains("collectionGroups"));
        assert!(json.contains("archivedCollections"));
    }

    #[test]
    fn missing_optional_sections_default() {
        let parsed: TaskFile = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert!(parsed.items.is_empty());
        assert!(parsed.collections.is_empty());
    }
}
```

- [ ] **Step 2: Wire the module** — append to `lib.rs`:

```rust
pub mod document;

pub use document::{TaskCollectionGroup, TaskFile};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p pond-core document`
Expected: PASS (3 tests).

- [ ] **Step 4: Lint + format**

Run: `cargo fmt && cargo clippy -p pond-core -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/pond-core/src/document.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add TaskFile on-disk document"
```

---

## Task 9: Store path resolution

**Files:**
- Create: `crates/pond-core/src/paths.rs`
- Modify: `crates/pond-core/src/lib.rs`

Mirrors `TaskStore.defaultStoreURL` / `appSupportDirectory`, cross-platform via `directories`. `POND_STORE` overrides the file path.

- [ ] **Step 1: Write the failing test** — create `crates/pond-core/src/paths.rs`

```rust
use directories::ProjectDirs;
use std::path::PathBuf;

const STORE_ENV: &str = "POND_STORE";

/// Directory for the data store and install record. macOS:
/// `~/Library/Application Support/pond/`, Linux: `~/.local/share/pond/`,
/// Windows: `%APPDATA%\pond\`.
pub fn data_directory() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("", "", "pond") {
        return dirs.data_dir().to_path_buf();
    }
    PathBuf::from("pond")
}

/// Resolved store path, honoring the `POND_STORE` override.
pub fn default_store_path() -> PathBuf {
    if let Ok(override_path) = std::env::var(STORE_ENV) {
        if !override_path.is_empty() {
            return PathBuf::from(override_path);
        }
    }
    data_directory().join("tasks.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_path_defaults_to_tasks_json() {
        // With the env unset, the file name is tasks.json under the data dir.
        // (Run serially elsewhere; here we only assert the suffix when unset.)
        if std::env::var(STORE_ENV).is_err() {
            assert!(default_store_path().ends_with("tasks.json"));
        }
    }

    #[test]
    fn data_directory_is_named_pond() {
        let dir = data_directory();
        assert_eq!(dir.file_name().unwrap().to_str().unwrap(), "pond");
    }
}
```

- [ ] **Step 2: Wire the module** — append to `lib.rs`:

```rust
pub mod paths;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p pond-core paths`
Expected: PASS (2 tests).

- [ ] **Step 4: Lint + format**

Run: `cargo fmt && cargo clippy -p pond-core -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/pond-core/src/paths.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add cross-platform store path resolution"
```

---

## Task 10: `TaskStore` with file lock + atomic IO

**Files:**
- Create: `crates/pond-core/src/store.rs`
- Modify: `crates/pond-core/src/lib.rs`

Mirrors `TaskStore.withFile(write:)`: an exclusive advisory lock on a sibling `.lock` file guards a read-modify-write; writes go through a temp file + atomic rename. This is the foundation every operation builds on.

- [ ] **Step 1: Write the failing test** — create `crates/pond-core/src/store.rs`

```rust
use crate::document::TaskFile;
use crate::error::{Result, StoreError};
use crate::json::to_pretty_sorted;
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

pub struct TaskStore {
    file_path: PathBuf,
    lock_path: PathBuf,
}

impl TaskStore {
    pub fn new<P: Into<PathBuf>>(file_path: P) -> Self {
        let file_path = file_path.into();
        let lock_path = with_extension_suffix(&file_path, "lock");
        TaskStore { file_path, lock_path }
    }

    pub fn open_default() -> Self {
        TaskStore::new(crate::paths::default_store_path())
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Run `body` under an exclusive lock. The (possibly mutated) file is written
    /// back atomically when `write` is true.
    pub(crate) fn with_file<T>(
        &self,
        write: bool,
        body: impl FnOnce(&mut TaskFile) -> Result<T>,
    ) -> Result<T> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&self.lock_path)
            .map_err(|e| StoreError::FileLockFailed(e.to_string()))?;
        lock_file
            .lock_exclusive()
            .map_err(|e| StoreError::FileLockFailed(e.to_string()))?;

        let result = (|| {
            let mut file = self.read_file()?;
            let value = body(&mut file)?;
            if write {
                self.write_file(&file)?;
            }
            Ok(value)
        })();

        let _ = lock_file.unlock();
        result
    }

    fn read_file(&self) -> Result<TaskFile> {
        match fs::read(&self.file_path) {
            Ok(bytes) if bytes.is_empty() => Ok(TaskFile::default()),
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(TaskFile::default()),
            Err(e) => Err(e.into()),
        }
    }

    fn write_file(&self, file: &TaskFile) -> Result<()> {
        let json = to_pretty_sorted(file)?;
        let tmp_path = with_extension_suffix(&self.file_path, "tmp");
        fs::write(&tmp_path, json.as_bytes())?;
        fs::rename(&tmp_path, &self.file_path)?;
        Ok(())
    }
}

fn with_extension_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn reads_default_when_missing() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let count = store.with_file(false, |f| Ok(f.items.len())).unwrap();
        assert_eq!(count, 0);
        assert!(!dir.path().join("tasks.json").exists(), "read must not create the file");
    }

    #[test]
    fn writes_then_reads_back() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .with_file(true, |f| {
                f.collections.push("Inbox".into());
                Ok(())
            })
            .unwrap();
        assert!(dir.path().join("tasks.json").exists());
        let collections = store.with_file(false, |f| Ok(f.collections.clone())).unwrap();
        assert_eq!(collections, vec!["Inbox".to_string()]);
    }

    #[test]
    fn read_only_does_not_persist() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.with_file(false, |f| { f.collections.push("Ghost".into()); Ok(()) }).unwrap();
        let collections = store.with_file(false, |f| Ok(f.collections.clone())).unwrap();
        assert!(collections.is_empty());
    }
}
```

- [ ] **Step 2: Wire the module** — append to `lib.rs`:

```rust
pub mod store;

pub use store::TaskStore;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p pond-core store`
Expected: PASS (3 tests).

- [ ] **Step 4: Lint + format**

Run: `cargo fmt && cargo clippy -p pond-core -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/pond-core/src/store.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add TaskStore with file lock and atomic writes"
```

---

## Task 11: Collection name normalization

**Files:**
- Create: `crates/pond-core/src/collections.rs`
- Modify: `crates/pond-core/src/lib.rs`

Mirrors the name helpers in `TaskCollectionSupport.swift`, minus legacy aliases. Constants: default collection `Inbox`, default group `DefaultGroup` (rendered "No Group" by the UI). A bare name lives in the default group; `Group/Name` names a grouped collection.

- [ ] **Step 1: Write the failing test** — create `crates/pond-core/src/collections.rs`

```rust
use crate::error::{Result, StoreError};

pub const DEFAULT_COLLECTION: &str = "Inbox";
pub const DEFAULT_GROUP: &str = "DefaultGroup";

pub struct CollectionReference {
    pub group_name: String,
    pub display_name: String,
}

impl CollectionReference {
    pub fn api_name(&self) -> String {
        collection_api_name(&self.group_name, &self.display_name)
    }
}

pub fn collection_api_name(group_name: &str, display_name: &str) -> String {
    if group_name == DEFAULT_GROUP {
        display_name.to_string()
    } else {
        format!("{group_name}/{display_name}")
    }
}

pub fn collection_display_name(collection: &str) -> String {
    match parse_reference(collection, DEFAULT_GROUP) {
        Ok(reference) => reference.display_name,
        Err(_) => collection.to_string(),
    }
}

pub fn collection_group_name_for_api(collection: &str) -> String {
    parse_reference(collection, DEFAULT_GROUP)
        .map(|r| r.group_name)
        .unwrap_or_else(|_| DEFAULT_GROUP.to_string())
}

/// Parse `Name` or `Group/Name` into a reference. Empty parts are invalid.
pub fn parse_reference(collection: &str, default_group: &str) -> Result<CollectionReference> {
    let default_group = normalized_explicit_group(default_group)?;
    let clean = collection.trim();
    if clean.is_empty() {
        return Err(StoreError::InvalidCollection);
    }
    let parts: Vec<&str> = clean.splitn(2, '/').collect();
    match parts.as_slice() {
        [name] => Ok(CollectionReference {
            group_name: default_group,
            display_name: normalized_display_name(name)?,
        }),
        [group, name] => Ok(CollectionReference {
            group_name: normalized_explicit_group(group)?,
            display_name: normalized_display_name(name)?,
        }),
        _ => Err(StoreError::InvalidCollection),
    }
}

fn normalized_display_name(display: &str) -> Result<String> {
    let clean = display.trim();
    if clean.is_empty() || clean.contains('/') {
        return Err(StoreError::InvalidCollection);
    }
    Ok(clean.to_string())
}

/// Empty/whitespace → default collection (`Inbox`); otherwise normalized api name.
pub fn normalized_collection(collection: &str) -> Result<String> {
    if collection.trim().is_empty() {
        return Ok(DEFAULT_COLLECTION.to_string());
    }
    Ok(parse_reference(collection, DEFAULT_GROUP)?.api_name())
}

/// Like `normalized_collection`, but empty is an error (used where a collection is required).
pub fn normalized_explicit_collection(collection: &str) -> Result<String> {
    if collection.trim().is_empty() {
        return Err(StoreError::InvalidCollection);
    }
    Ok(parse_reference(collection, DEFAULT_GROUP)?.api_name())
}

pub fn normalized_explicit_group(group: &str) -> Result<String> {
    let clean = group.trim();
    if clean.is_empty() || clean.contains('/') {
        return Err(StoreError::InvalidCollectionGroup);
    }
    Ok(clean.to_string())
}

/// Dedup a list of collection names, preserving first-seen order; the default
/// collection (if present) sorts first is handled elsewhere by `sorted_collection_names`.
pub fn normalized_collection_list<I: IntoIterator<Item = String>>(collections: I) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for collection in collections {
        let clean = normalized_collection(&collection).unwrap_or_else(|_| DEFAULT_COLLECTION.to_string());
        if seen.insert(clean.clone()) {
            result.push(clean);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_name_is_default_group() {
        let r = parse_reference("Errands", DEFAULT_GROUP).unwrap();
        assert_eq!(r.group_name, DEFAULT_GROUP);
        assert_eq!(r.display_name, "Errands");
        assert_eq!(r.api_name(), "Errands");
    }

    #[test]
    fn grouped_name_round_trips() {
        let r = parse_reference("Work/Tasks", DEFAULT_GROUP).unwrap();
        assert_eq!(r.group_name, "Work");
        assert_eq!(r.display_name, "Tasks");
        assert_eq!(r.api_name(), "Work/Tasks");
        assert_eq!(collection_display_name("Work/Tasks"), "Tasks");
        assert_eq!(collection_group_name_for_api("Work/Tasks"), "Work");
        assert_eq!(collection_group_name_for_api("Errands"), DEFAULT_GROUP);
    }

    #[test]
    fn empty_rules() {
        assert_eq!(normalized_collection("  ").unwrap(), "Inbox");
        assert_eq!(normalized_explicit_collection("  ").unwrap_err(), StoreError::InvalidCollection);
        // splitn(2, '/') keeps the remainder, so "a/b/c" → group "a", display "b/c";
        // a display name containing '/' is rejected.
        assert_eq!(parse_reference("a/b/c", DEFAULT_GROUP).unwrap_err(), StoreError::InvalidCollection);
    }

    #[test]
    fn list_dedups_and_normalizes() {
        let list = normalized_collection_list(vec!["Inbox".into(), "Inbox".into(), "Work/A".into()]);
        assert_eq!(list, vec!["Inbox".to_string(), "Work/A".to_string()]);
    }
}
```

- [ ] **Step 2: Wire the module** — append to `lib.rs`:

```rust
pub mod collections;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p pond-core collections`
Expected: PASS (4 tests).

- [ ] **Step 4: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/collections.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add collection name normalization"
```

---

## Task 12: Group normalization & in-file helpers

**Files:**
- Modify: `crates/pond-core/src/collections.rs`

Mirrors `normalizedCollectionGroups`, `addCollectionIfMissing`, `moveCollectionInFile`, `removeCollectionFromGroups`, `collectionExists`, `collectionGroupName(containing:)`. The default group is always present and sorts first; unassigned collections are placed in their api-name group.

- [ ] **Step 1: Write the failing test** — add to `crates/pond-core/src/collections.rs` (above `mod tests`):

```rust
use crate::document::{TaskCollectionGroup, TaskFile};
use crate::model::CollectionColor;

pub fn collection_exists(collection: &str, file: &TaskFile) -> bool {
    file.collections.iter().any(|c| c == collection)
        || file.items.iter().any(|i| i.collection == collection)
}

pub fn collection_group_containing(collection: &str, file: &TaskFile) -> Option<String> {
    normalized_collection_groups(&file.collection_groups, &all_collection_names(file))
        .into_iter()
        .find(|g| g.collections.iter().any(|c| c == collection))
        .map(|g| g.name)
}

pub(crate) fn all_collection_names(file: &TaskFile) -> Vec<String> {
    let mut names = file.collections.clone();
    names.extend(file.items.iter().map(|i| i.collection.clone()));
    normalized_collection_list(names)
}

/// Rebuild groups: dedup names, keep the default group first, assign every known
/// collection to exactly one group (its api-name group if otherwise unassigned).
pub fn normalized_collection_groups(
    groups: &[TaskCollectionGroup],
    collections: &[String],
) -> Vec<TaskCollectionGroup> {
    let names = normalized_collection_list(collections.to_vec());
    let name_set: std::collections::HashSet<&String> = names.iter().collect();
    let mut seen_groups = std::collections::HashSet::new();
    let mut assigned = std::collections::HashSet::new();
    let mut result: Vec<TaskCollectionGroup> = Vec::new();

    for group in groups {
        let clean_name = group.name.trim();
        if clean_name.is_empty() || seen_groups.contains(clean_name) {
            continue;
        }
        let clean_collections: Vec<String> = normalized_collection_list(
            group.collections.iter().map(|c| {
                collection_api_name(clean_name, &collection_display_name(c))
            }).collect::<Vec<_>>(),
        )
        .into_iter()
        .filter(|c| name_set.contains(c) && !assigned.contains(c))
        .collect();
        for c in &clean_collections {
            assigned.insert(c.clone());
        }
        seen_groups.insert(clean_name.to_string());
        result.push(TaskCollectionGroup { name: clean_name.to_string(), collections: clean_collections });
    }

    if !seen_groups.contains(DEFAULT_GROUP) {
        result.insert(0, TaskCollectionGroup { name: DEFAULT_GROUP.to_string(), collections: Vec::new() });
        seen_groups.insert(DEFAULT_GROUP.to_string());
    }

    for collection in names.iter().filter(|c| !assigned.contains(*c)) {
        let group_name = collection_group_name_for_api(collection);
        if let Some(group) = result.iter_mut().find(|g| g.name == group_name) {
            group.collections.push(collection.clone());
            group.collections = normalized_collection_list(group.collections.clone());
        } else {
            result.push(TaskCollectionGroup { name: group_name, collections: vec![collection.clone()] });
        }
    }

    // Default group first, then the rest in encountered order.
    let mut ordered: Vec<TaskCollectionGroup> = result.iter().filter(|g| g.name == DEFAULT_GROUP).cloned().collect();
    ordered.extend(result.into_iter().filter(|g| g.name != DEFAULT_GROUP));
    ordered
}

pub fn normalize_groups_in_file(file: &mut TaskFile) {
    let names = all_collection_names(file);
    file.collection_groups = normalized_collection_groups(&file.collection_groups, &names);
}

pub fn remove_collection_from_groups(collection: &str, file: &mut TaskFile) {
    for group in &mut file.collection_groups {
        group.collections.retain(|c| c != collection);
    }
}

pub fn add_collection_group_if_missing(group: &str, file: &mut TaskFile) {
    normalize_groups_in_file(file);
    if !file.collection_groups.iter().any(|g| g.name == group) {
        file.collection_groups.push(TaskCollectionGroup { name: group.to_string(), collections: Vec::new() });
    }
}

pub fn move_collection_in_file(collection: &str, group: &str, file: &mut TaskFile) {
    add_collection_group_if_missing(group, file);
    remove_collection_from_groups(collection, file);
    if let Some(target) = file.collection_groups.iter_mut().find(|g| g.name == group) {
        target.collections.push(collection.to_string());
        target.collections = normalized_collection_list(target.collections.clone());
    }
    normalize_groups_in_file(file);
}

/// Ensure a collection exists (and is colored gray by default), placing it in `group`
/// when given or in its api-name group when newly added.
pub fn add_collection_if_missing(collection: &str, group: Option<&str>, file: &mut TaskFile) -> Result<()> {
    let clean = normalized_collection(collection)?;
    let resolved_group = match group {
        Some(g) => normalized_explicit_group(g)?,
        None => collection_group_name_for_api(&clean),
    };
    let already = collection_exists(&clean, file);
    file.collections = normalized_collection_list(file.collections.iter().cloned().chain([clean.clone()]).collect::<Vec<_>>());
    file.collection_colors.entry(clean.clone()).or_insert(CollectionColor::Gray);

    if group.is_some() {
        move_collection_in_file(&clean, &resolved_group, file);
    } else if !already && collection_group_containing(&clean, file).is_none() {
        move_collection_in_file(&clean, &resolved_group, file);
    } else {
        normalize_groups_in_file(file);
    }
    Ok(())
}
```

Then add to `mod tests`:

```rust
    use crate::document::{TaskCollectionGroup, TaskFile};

    #[test]
    fn default_group_is_always_present_and_first() {
        let groups = normalized_collection_groups(&[], &[]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, DEFAULT_GROUP);
    }

    #[test]
    fn unassigned_collections_land_in_their_group() {
        let groups = normalized_collection_groups(&[], &["Inbox".into(), "Work/A".into()]);
        let default = groups.iter().find(|g| g.name == DEFAULT_GROUP).unwrap();
        assert!(default.collections.contains(&"Inbox".to_string()));
        let work = groups.iter().find(|g| g.name == "Work").unwrap();
        assert_eq!(work.collections, vec!["Work/A".to_string()]);
    }

    #[test]
    fn add_collection_if_missing_colors_gray_and_groups() {
        let mut file = TaskFile::default();
        add_collection_if_missing("Work/A", None, &mut file).unwrap();
        assert!(collection_exists("Work/A", &file));
        assert_eq!(file.collection_colors.get("Work/A"), Some(&CollectionColor::Gray));
        assert_eq!(collection_group_containing("Work/A", &file).as_deref(), Some("Work"));
    }

    #[test]
    fn group_membership_follows_api_name() {
        // A collection listed under a group that doesn't match its API name is re-homed
        // to the group its API name implies. "A" is a bare (default-group) API name, so
        // even when listed under "Work" it normalizes into DefaultGroup. (Real relocation
        // renames the API name first — see Task 20's move_collection.)
        let groups = vec![TaskCollectionGroup {
            name: "Work".into(),
            collections: vec!["A".into()],
        }];
        let normalized = normalized_collection_groups(&groups, &["A".into()]);
        let default = normalized.iter().find(|g| g.name == DEFAULT_GROUP).unwrap();
        assert!(default.collections.contains(&"A".to_string()));
        assert!(normalized
            .iter()
            .find(|g| g.name == "Work")
            .map_or(true, |g| !g.collections.contains(&"A".to_string())));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p pond-core collections`
Expected: PASS (previous 4 + new 4).

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/collections.rs
git commit -m "feat(core): add collection group normalization helpers"
```

---

## Task 13: Summary builders

**Files:**
- Modify: `crates/pond-core/src/collections.rs`

Mirrors `makeCollectionSummaries`, `makeCollectionGroupSummaries`, `collectionSummary`, `collectionStatusIndicator`, `sortedCollectionNames`. The default collection sorts first; the status indicator precedence is aborted ▸ rejected ▸ on-hold.

- [ ] **Step 1: Write the failing test** — add to `crates/pond-core/src/collections.rs` (above `mod tests`):

```rust
use crate::model::{CollectionGroupSummary, CollectionSummary, TaskItem, TaskStatus};

pub fn sorted_collection_names<I: IntoIterator<Item = String>>(collections: I) -> Vec<String> {
    let mut names = normalized_collection_list(collections.into_iter().collect::<Vec<_>>());
    names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    let mut ordered: Vec<String> = names.iter().filter(|n| *n == DEFAULT_COLLECTION).cloned().collect();
    ordered.extend(names.into_iter().filter(|n| n != DEFAULT_COLLECTION));
    ordered
}

pub fn collection_status_indicator(items: &[TaskItem]) -> Option<TaskStatus> {
    if items.iter().any(|i| i.status == TaskStatus::Aborted) {
        Some(TaskStatus::Aborted)
    } else if items.iter().any(|i| i.status == TaskStatus::Rejected) {
        Some(TaskStatus::Rejected)
    } else if items.iter().any(|i| i.status == TaskStatus::OnHold) {
        Some(TaskStatus::OnHold)
    } else {
        None
    }
}

pub fn collection_summary(name: &str, file: &TaskFile) -> CollectionSummary {
    let items: Vec<&TaskItem> = file.items.iter().filter(|i| i.collection == name).collect();
    let group_name = collection_group_containing(name, file)
        .unwrap_or_else(|| collection_group_name_for_api(name));
    CollectionSummary {
        name: name.to_string(),
        display_name: collection_display_name(name),
        group_name,
        total_count: items.len(),
        incomplete_count: items.iter().filter(|i| i.status.is_incomplete()).count(),
        status_indicator: collection_status_indicator(
            &items.iter().map(|i| (*i).clone()).collect::<Vec<_>>(),
        ),
        color: file.collection_colors.get(name).copied().unwrap_or_default(),
        is_archived: file.archived_collections.contains(name),
        prompt_template: file.collection_prompts.get(name).cloned(),
    }
}

pub fn make_collection_summaries(file: &TaskFile) -> Vec<CollectionSummary> {
    let mut names = file.collections.clone();
    names.extend(file.items.iter().map(|i| i.collection.clone()));
    sorted_collection_names(names)
        .into_iter()
        .map(|name| collection_summary(&name, file))
        .collect()
}

pub fn make_collection_group_summaries(file: &TaskFile) -> Vec<CollectionGroupSummary> {
    let summaries = make_collection_summaries(file);
    // Preserve the sorted order from make_collection_summaries (HashMap key order is
    // non-deterministic and would make intra-group ordering unstable).
    let names: Vec<String> = summaries.iter().map(|s| s.name.clone()).collect();
    let by_name: std::collections::HashMap<String, CollectionSummary> =
        summaries.into_iter().map(|s| (s.name.clone(), s)).collect();
    normalized_collection_groups(&file.collection_groups, &names)
        .into_iter()
        .map(|group| CollectionGroupSummary {
            name: group.name,
            collections: group
                .collections
                .into_iter()
                .filter_map(|c| by_name.get(&c).cloned())
                .collect(),
        })
        .collect()
}
```

Then add to `mod tests`:

```rust
    use crate::model::{CollectionColor as Color, TaskStatus as S};
    use chrono::{TimeZone, Utc};

    fn item(id: &str, collection: &str, status: S) -> TaskItem {
        let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();
        let mut it = TaskItem::new(id.into(), "t".into(), collection.into(), status, now);
        it.version = "v".repeat(12);
        it
    }

    #[test]
    fn default_collection_sorts_first() {
        let names = sorted_collection_names(vec!["Zebra".into(), "Inbox".into(), "Apple".into()]);
        assert_eq!(names, vec!["Inbox".to_string(), "Apple".to_string(), "Zebra".to_string()]);
    }

    #[test]
    fn status_indicator_precedence() {
        let items = vec![item("00000001", "A", S::OnHold), item("00000002", "A", S::Aborted)];
        assert_eq!(collection_status_indicator(&items), Some(S::Aborted));
    }

    #[test]
    fn summary_counts_incomplete() {
        let mut file = TaskFile::default();
        file.items.push(item("00000001", "Inbox", S::Ready));
        file.items.push(item("00000002", "Inbox", S::Completed));
        file.collection_colors.insert("Inbox".into(), Color::Blue);
        let s = collection_summary("Inbox", &file);
        assert_eq!(s.total_count, 2);
        assert_eq!(s.incomplete_count, 1);
        assert_eq!(s.color, Color::Blue);
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p pond-core collections`
Expected: PASS.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/collections.rs
git commit -m "feat(core): add collection summary builders"
```

---

## Task 14: Item queries

**Files:**
- Modify: `crates/pond-core/src/store.rs`

Mirrors `resolveIndex`, `TaskStore.items`, `collectionSummaries`, `collectionGroupSummaries`. `resolve_index` matches an exact id, else a unique prefix, else errors.

- [ ] **Step 1: Write the failing test** — add to `crates/pond-core/src/store.rs` (above its `mod tests`):

```rust
use crate::collections::{make_collection_group_summaries, make_collection_summaries, normalized_collection};
use crate::model::{CollectionGroupSummary, CollectionSummary, TaskItem, TaskStatus};

pub(crate) fn resolve_index(id: &str, items: &[TaskItem]) -> Result<usize> {
    if let Some(i) = items.iter().position(|it| it.id == id) {
        return Ok(i);
    }
    let matches: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, it)| it.id.starts_with(id))
        .map(|(i, _)| i)
        .collect();
    match matches.as_slice() {
        [] => Err(StoreError::NotFound(id.to_string())),
        [only] => Ok(*only),
        many => Err(StoreError::AmbiguousId(
            id.to_string(),
            many.iter().map(|i| items[*i].id.clone()).collect(),
        )),
    }
}

impl TaskStore {
    pub fn items(
        &self,
        status: Option<TaskStatus>,
        collection: Option<&str>,
        ids: &[String],
        search: Option<&str>,
    ) -> Result<Vec<TaskItem>> {
        let clean_collection = match collection {
            Some(c) => Some(normalized_collection(c)?),
            None => None,
        };
        self.with_file(false, |file| {
            let mut results: Vec<TaskItem> = if ids.is_empty() {
                file.items.clone()
            } else {
                ids.iter()
                    .map(|id| resolve_index(id, &file.items).map(|i| file.items[i].clone()))
                    .collect::<Result<Vec<_>>>()?
            };
            if let Some(status) = status {
                results.retain(|i| i.status == status);
            }
            if let Some(ref c) = clean_collection {
                results.retain(|i| &i.collection == c);
            }
            if let Some(query) = search.map(str::trim).filter(|q| !q.is_empty()) {
                let q = query.to_lowercase();
                results.retain(|i| {
                    i.title.to_lowercase().contains(&q)
                        || i.collection.to_lowercase().contains(&q)
                        || i.id.to_lowercase().contains(&q)
                        || i.note.as_ref().map_or(false, |n| n.body.to_lowercase().contains(&q))
                });
            }
            Ok(results)
        })
    }

    pub fn collection_summaries(&self) -> Result<Vec<CollectionSummary>> {
        self.with_file(false, |file| Ok(make_collection_summaries(file)))
    }

    pub fn collection_group_summaries(&self) -> Result<Vec<CollectionGroupSummary>> {
        self.with_file(false, |file| Ok(make_collection_group_summaries(file)))
    }
}
```

Then add to the store `mod tests`:

```rust
    use crate::model::TaskStatus;
    use chrono::{TimeZone, Utc};

    fn seed(store: &TaskStore, items: &[(&str, &str, TaskStatus)]) {
        store.with_file(true, |f| {
            let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();
            for (id, collection, status) in items {
                let mut it = TaskItem::new((*id).into(), "t".into(), (*collection).into(), *status, now);
                it.version = "v".repeat(12);
                f.items.push(it);
                f.collections = crate::collections::normalized_collection_list(
                    f.collections.iter().cloned().chain([(*collection).to_string()]).collect::<Vec<_>>(),
                );
            }
            Ok(())
        }).unwrap();
    }

    #[test]
    fn filters_by_status_and_collection() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        seed(&store, &[("00000001", "Inbox", TaskStatus::Ready), ("00000002", "Work/A", TaskStatus::Completed)]);
        assert_eq!(store.items(Some(TaskStatus::Ready), None, &[], None).unwrap().len(), 1);
        assert_eq!(store.items(None, Some("Work/A"), &[], None).unwrap().len(), 1);
    }

    #[test]
    fn resolves_id_by_unique_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        seed(&store, &[("0123abcd", "Inbox", TaskStatus::Ready)]);
        let found = store.items(None, None, &["0123".to_string()], None).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "0123abcd");
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p pond-core store`
Expected: PASS (previous 3 + new 2).

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/store.rs
git commit -m "feat(core): add item queries and summary accessors"
```

---

## Task 15: Create items

**Files:**
- Modify: `crates/pond-core/src/store.rs`

Mirrors `TaskStore.add`. New tasks default to `Ready`. Title is trimmed unless `allow_empty_title`. The id must be valid and unique; the collection is auto-created.

- [ ] **Step 1: Write the failing test** — add to `store.rs` (inside a new `impl TaskStore` block above `mod tests`):

```rust
use crate::collections::add_collection_if_missing;
use crate::ids::{is_valid_id, make_id, make_version};
use chrono::Utc;
use std::collections::HashSet;

impl TaskStore {
    pub fn add(
        &self,
        title: &str,
        collection: &str,
        requested_id: Option<&str>,
        allow_empty_title: bool,
        status: TaskStatus,
    ) -> Result<TaskItem> {
        let clean_title = if allow_empty_title { title.to_string() } else { title.trim().to_string() };
        let clean_collection = normalized_collection(collection)?;
        if !allow_empty_title && clean_title.is_empty() {
            return Err(StoreError::InvalidTitle);
        }
        self.with_file(true, |file| {
            let now = Utc::now();
            let existing: HashSet<String> = file.items.iter().map(|i| i.id.clone()).collect();
            let id = match requested_id {
                Some(id) => id.to_string(),
                None => make_id(&existing),
            };
            if !is_valid_id(&id) {
                return Err(StoreError::InvalidId(id));
            }
            if existing.contains(&id) {
                return Err(StoreError::DuplicateId(id));
            }
            let mut item = TaskItem::new(id, clean_title.clone(), clean_collection.clone(), status, now);
            item.version = make_version(&file.items.iter().map(|i| i.version.clone()).collect());
            file.items.push(item.clone());
            add_collection_if_missing(&item.collection, None, file)?;
            Ok(item)
        })
    }
}
```

Add to `mod tests`:

```rust
    #[test]
    fn add_trims_and_defaults_collection() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let item = store.add("  Buy milk  ", "", None, false, TaskStatus::Ready).unwrap();
        assert_eq!(item.title, "Buy milk");
        assert_eq!(item.collection, "Inbox");
        assert!(is_valid_id(&item.id));
    }

    #[test]
    fn add_rejects_empty_title_unless_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        assert_eq!(store.add("   ", "Inbox", None, false, TaskStatus::Ready).unwrap_err(), StoreError::InvalidTitle);
        let empty = store.add("", "Inbox", None, true, TaskStatus::Draft).unwrap();
        assert_eq!(empty.title, "");
        assert_eq!(empty.status, TaskStatus::Draft);
    }

    #[test]
    fn add_rejects_duplicate_and_invalid_ids() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("0123abcd"), false, TaskStatus::Ready).unwrap();
        assert_eq!(store.add("b", "Inbox", Some("0123abcd"), false, TaskStatus::Ready).unwrap_err(), StoreError::DuplicateId("0123abcd".into()));
        assert_eq!(store.add("c", "Inbox", Some("xyz"), false, TaskStatus::Ready).unwrap_err(), StoreError::InvalidId("xyz".into()));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p pond-core store`
Expected: PASS.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/store.rs
git commit -m "feat(core): add item creation"
```

---

## Task 16: Update, title, move (+ version bump)

**Files:**
- Modify: `crates/pond-core/src/store.rs`

Mirrors `applyUpdate`, `markItemUpdated`/`refreshVersion`, `update`, `updateTitle`, `move`, and their `if_current` variants. A change bumps `updated_at` and `version`. `if_current` is the optimistic-concurrency guard (Swift's `ifCurrent`): it applies only when the stored item still equals the expected one.

- [ ] **Step 1: Write the failing test** — add to `store.rs` (new `impl` block above `mod tests`):

```rust
impl TaskStore {
    fn mark_updated(file: &mut TaskFile, index: usize, now: chrono::DateTime<Utc>) {
        file.items[index].updated_at = now;
        let mut existing: HashSet<String> = file.items.iter().map(|i| i.version.clone()).collect();
        existing.remove(&file.items[index].version);
        file.items[index].version = make_version(&existing);
    }

    fn apply_update(
        file: &mut TaskFile,
        index: usize,
        title: Option<&str>,
        collection: Option<&str>,
        status: Option<TaskStatus>,
    ) -> Result<bool> {
        let mut changed = false;
        if let Some(title) = title {
            if file.items[index].title != title {
                file.items[index].title = title.to_string();
                changed = true;
            }
        }
        if let Some(collection) = collection {
            add_collection_if_missing(collection, None, file)?;
            if file.items[index].collection != collection {
                file.items[index].collection = collection.to_string();
                changed = true;
            }
        }
        if let Some(status) = status {
            if file.items[index].status != status {
                file.items[index].status = status;
                changed = true;
            }
        }
        Ok(changed)
    }

    pub fn update(
        &self,
        id: &str,
        title: Option<&str>,
        collection: Option<&str>,
        status: Option<TaskStatus>,
    ) -> Result<TaskItem> {
        let clean_collection = match collection {
            Some(c) => Some(normalized_collection(c)?),
            None => None,
        };
        if title.is_none() && clean_collection.is_none() && status.is_none() {
            return Err(StoreError::MissingUpdate);
        }
        self.with_file(true, |file| {
            let index = resolve_index(id, &file.items)?;
            if Self::apply_update(file, index, title, clean_collection.as_deref(), status)? {
                Self::mark_updated(file, index, Utc::now());
            }
            Ok(file.items[index].clone())
        })
    }

    pub fn update_if_current(
        &self,
        id: &str,
        title: Option<&str>,
        collection: Option<&str>,
        status: Option<TaskStatus>,
        expected: &TaskItem,
    ) -> Result<Option<TaskItem>> {
        let clean_collection = match collection {
            Some(c) => Some(normalized_collection(c)?),
            None => None,
        };
        if title.is_none() && clean_collection.is_none() && status.is_none() {
            return Err(StoreError::MissingUpdate);
        }
        self.with_file(true, |file| {
            let index = resolve_index(id, &file.items)?;
            if &file.items[index] != expected {
                return Ok(None);
            }
            if Self::apply_update(file, index, title, clean_collection.as_deref(), status)? {
                Self::mark_updated(file, index, Utc::now());
            }
            Ok(Some(file.items[index].clone()))
        })
    }

    pub fn move_item(&self, id: &str, collection: &str) -> Result<TaskItem> {
        let clean = normalized_collection(collection)?;
        self.with_file(true, |file| {
            let index = resolve_index(id, &file.items)?;
            add_collection_if_missing(&clean, None, file)?;
            if file.items[index].collection != clean {
                file.items[index].collection = clean.clone();
                Self::mark_updated(file, index, Utc::now());
            }
            Ok(file.items[index].clone())
        })
    }
}
```

Add to `mod tests`:

```rust
    #[test]
    fn update_bumps_version_and_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let created = store.add("a", "Inbox", Some("0123abcd"), false, TaskStatus::Ready).unwrap();
        let updated = store.update("0123abcd", Some("a2"), None, None).unwrap();
        assert_eq!(updated.title, "a2");
        assert_ne!(updated.version, created.version);
    }

    #[test]
    fn update_requires_a_field() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("0123abcd"), false, TaskStatus::Ready).unwrap();
        assert_eq!(store.update("0123abcd", None, None, None).unwrap_err(), StoreError::MissingUpdate);
    }

    #[test]
    fn if_current_skips_on_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let created = store.add("a", "Inbox", Some("0123abcd"), false, TaskStatus::Ready).unwrap();
        store.update("0123abcd", Some("changed"), None, None).unwrap(); // now stale
        let result = store.update_if_current("0123abcd", Some("x"), None, None, &created).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn move_changes_collection() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("0123abcd"), false, TaskStatus::Ready).unwrap();
        let moved = store.move_item("0123abcd", "Work/A").unwrap();
        assert_eq!(moved.collection, "Work/A");
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p pond-core store`
Expected: PASS.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/store.rs
git commit -m "feat(core): add item update and move with version bump"
```

---

## Task 17: Delete, clear, status changes

**Files:**
- Modify: `crates/pond-core/src/store.rs`

Mirrors `delete`, `delete(ids/collection)`, `clearItems`, `setStatus`, `setStatuses` (+ `if_current` for single-item delete and status). `targetCollection` enforces "ids XOR collection".

- [ ] **Step 1: Write the failing test** — add to `store.rs` (new `impl` block):

```rust
use std::collections::HashMap;

impl TaskStore {
    fn target_collection(ids: &[String], collection: Option<&str>) -> Result<Option<String>> {
        let clean = match collection {
            Some(c) => Some(crate::collections::normalized_explicit_collection(c)?),
            None => None,
        };
        if ids.is_empty() && clean.is_none() {
            return Err(StoreError::MissingTarget);
        }
        if !ids.is_empty() && clean.is_some() {
            return Err(StoreError::TargetConflict);
        }
        Ok(clean)
    }

    fn target_indexes(file: &TaskFile, ids: &[String], collection: &Option<String>) -> Result<Vec<usize>> {
        let mut indexes: Vec<usize> = if let Some(c) = collection {
            file.items.iter().enumerate().filter(|(_, i)| &i.collection == c).map(|(i, _)| i).collect()
        } else {
            ids.iter().map(|id| resolve_index(id, &file.items)).collect::<Result<Vec<_>>>()?
        };
        indexes.sort_unstable();
        indexes.dedup();
        if indexes.is_empty() {
            return Err(StoreError::NoMatchingTasks);
        }
        Ok(indexes)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.with_file(true, |file| {
            let index = resolve_index(id, &file.items)?;
            file.items.remove(index);
            Ok(())
        })
    }

    pub fn delete_if_current(&self, id: &str, expected: &TaskItem) -> Result<bool> {
        self.with_file(true, |file| {
            let index = resolve_index(id, &file.items)?;
            if &file.items[index] != expected {
                return Ok(false);
            }
            file.items.remove(index);
            Ok(true)
        })
    }

    pub fn delete_many(&self, ids: &[String], collection: Option<&str>) -> Result<Vec<TaskItem>> {
        let clean = Self::target_collection(ids, collection)?;
        self.with_file(true, |file| {
            let mut indexes = Self::target_indexes(file, ids, &clean)?;
            let deleted: Vec<TaskItem> = indexes.iter().map(|i| file.items[*i].clone()).collect();
            indexes.sort_unstable_by(|a, b| b.cmp(a));
            for i in indexes {
                file.items.remove(i);
            }
            Ok(deleted)
        })
    }

    pub fn clear_items(&self, collection: &str, completed_only: bool) -> Result<Vec<TaskItem>> {
        let clean = crate::collections::normalized_explicit_collection(collection)?;
        self.with_file(true, |file| {
            let mut indexes: Vec<usize> = file
                .items
                .iter()
                .enumerate()
                .filter(|(_, i)| i.collection == clean && (!completed_only || i.status == TaskStatus::Completed))
                .map(|(i, _)| i)
                .collect();
            if indexes.is_empty() {
                return Err(StoreError::NoMatchingTasks);
            }
            let deleted: Vec<TaskItem> = indexes.iter().map(|i| file.items[*i].clone()).collect();
            indexes.sort_unstable_by(|a, b| b.cmp(a));
            for i in indexes {
                file.items.remove(i);
            }
            Ok(deleted)
        })
    }

    pub fn set_status(&self, status: TaskStatus, ids: &[String], collection: Option<&str>) -> Result<Vec<TaskItem>> {
        let clean = Self::target_collection(ids, collection)?;
        self.with_file(true, |file| {
            let indexes = Self::target_indexes(file, ids, &clean)?;
            let now = Utc::now();
            for i in &indexes {
                if file.items[*i].status != status {
                    file.items[*i].status = status;
                    Self::mark_updated(file, *i, now);
                }
            }
            Ok(indexes.iter().map(|i| file.items[*i].clone()).collect())
        })
    }

    pub fn set_status_if_current(&self, status: TaskStatus, id: &str, expected: &TaskItem) -> Result<Option<TaskItem>> {
        self.with_file(true, |file| {
            let index = resolve_index(id, &file.items)?;
            if &file.items[index] != expected {
                return Ok(None);
            }
            if file.items[index].status != status {
                file.items[index].status = status;
                Self::mark_updated(file, index, Utc::now());
            }
            Ok(Some(file.items[index].clone()))
        })
    }

    pub fn set_statuses(
        &self,
        replacements: &HashMap<TaskStatus, TaskStatus>,
        ids: &[String],
        collection: Option<&str>,
    ) -> Result<Vec<TaskItem>> {
        let clean = Self::target_collection(ids, collection)?;
        let meaningful: HashMap<TaskStatus, TaskStatus> =
            replacements.iter().filter(|(a, b)| a != b).map(|(a, b)| (*a, *b)).collect();
        self.with_file(true, |file| {
            let all = Self::target_indexes(file, ids, &clean)?;
            let indexes: Vec<usize> = all.into_iter().filter(|i| meaningful.contains_key(&file.items[*i].status)).collect();
            if indexes.is_empty() {
                return Ok(Vec::new());
            }
            let now = Utc::now();
            for i in &indexes {
                if let Some(replacement) = meaningful.get(&file.items[*i].status).copied() {
                    file.items[*i].status = replacement;
                    Self::mark_updated(file, *i, now);
                }
            }
            Ok(indexes.iter().map(|i| file.items[*i].clone()).collect())
        })
    }
}
```

Add to `mod tests`:

```rust
    #[test]
    fn delete_many_requires_exactly_one_target() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("00000001"), false, TaskStatus::Ready).unwrap();
        assert_eq!(store.delete_many(&[], None).unwrap_err(), StoreError::MissingTarget);
        assert_eq!(store.delete_many(&["00000001".into()], Some("Inbox")).unwrap_err(), StoreError::TargetConflict);
        assert_eq!(store.delete_many(&["00000001".into()], None).unwrap().len(), 1);
    }

    #[test]
    fn clear_completed_only() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("00000001"), false, TaskStatus::Completed).unwrap();
        store.add("b", "Inbox", Some("00000002"), false, TaskStatus::Ready).unwrap();
        let cleared = store.clear_items("Inbox", true).unwrap();
        assert_eq!(cleared.len(), 1);
        assert_eq!(store.items(None, Some("Inbox"), &[], None).unwrap().len(), 1);
    }

    #[test]
    fn set_status_updates_matching() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("00000001"), false, TaskStatus::Ready).unwrap();
        let changed = store.set_status(TaskStatus::Completed, &["00000001".into()], None).unwrap();
        assert_eq!(changed[0].status, TaskStatus::Completed);
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p pond-core store`
Expected: PASS.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/store.rs
git commit -m "feat(core): add delete, clear, and status operations"
```

---

## Task 18: Notes

**Files:**
- Modify: `crates/pond-core/src/store.rs`

Mirrors `addNote`, `updateNote`, `deleteNote` (+ `if_current`). A note body is trimmed and must be non-empty. Adding replaces the single note; deleting clears it. Note edits bump the item version.

- [ ] **Step 1: Write the failing test** — add to `store.rs` (new `impl` block):

```rust
use crate::model::TaskNote;

impl TaskStore {
    fn put_note(item: &mut TaskItem, body: &str, now: chrono::DateTime<Utc>) {
        let id = item.note.as_ref().map(|n| n.id.clone()).unwrap_or_else(|| make_id(&HashSet::new()));
        item.note = Some(TaskNote {
            id,
            version: make_version(&HashSet::new()),
            body: body.to_string(),
            created_at: now,
            updated_at: now,
        });
    }

    pub fn add_note(&self, id: &str, body: &str) -> Result<TaskItem> {
        let clean = body.trim();
        if clean.is_empty() {
            return Err(StoreError::InvalidNote);
        }
        self.with_file(true, |file| {
            let index = resolve_index(id, &file.items)?;
            let now = Utc::now();
            Self::put_note(&mut file.items[index], clean, now);
            Self::mark_updated(file, index, now);
            Ok(file.items[index].clone())
        })
    }

    pub fn add_note_if_current(&self, id: &str, body: &str, expected: &TaskItem) -> Result<Option<TaskItem>> {
        let clean = body.trim();
        if clean.is_empty() {
            return Err(StoreError::InvalidNote);
        }
        self.with_file(true, |file| {
            let index = resolve_index(id, &file.items)?;
            if &file.items[index] != expected {
                return Ok(None);
            }
            let now = Utc::now();
            Self::put_note(&mut file.items[index], clean, now);
            Self::mark_updated(file, index, now);
            Ok(Some(file.items[index].clone()))
        })
    }

    pub fn update_note(&self, id: &str, body: &str) -> Result<TaskItem> {
        let clean = body.trim();
        if clean.is_empty() {
            return Err(StoreError::InvalidNote);
        }
        self.with_file(true, |file| {
            let index = resolve_index(id, &file.items)?;
            let changed = match file.items[index].note.as_mut() {
                Some(note) if note.body != clean => {
                    note.body = clean.to_string();
                    note.updated_at = Utc::now();
                    note.version = make_version(&HashSet::new());
                    true
                }
                Some(_) => false,
                None => return Err(StoreError::NoteNotFound(id.to_string())),
            };
            if changed {
                Self::mark_updated(file, index, Utc::now());
            }
            Ok(file.items[index].clone())
        })
    }

    pub fn delete_note(&self, id: &str) -> Result<TaskItem> {
        self.with_file(true, |file| {
            let index = resolve_index(id, &file.items)?;
            if file.items[index].note.is_none() {
                return Err(StoreError::NoteNotFound(id.to_string()));
            }
            file.items[index].note = None;
            Self::mark_updated(file, index, Utc::now());
            Ok(file.items[index].clone())
        })
    }

    pub fn delete_note_if_current(&self, id: &str, expected: &TaskItem) -> Result<Option<TaskItem>> {
        self.with_file(true, |file| {
            let index = resolve_index(id, &file.items)?;
            if &file.items[index] != expected {
                return Ok(None);
            }
            file.items[index].note = None;
            Self::mark_updated(file, index, Utc::now());
            Ok(Some(file.items[index].clone()))
        })
    }
}
```

Add to `mod tests`:

```rust
    #[test]
    fn add_then_update_then_delete_note() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("00000001"), false, TaskStatus::Ready).unwrap();
        let with_note = store.add_note("00000001", "  hello  ").unwrap();
        assert_eq!(with_note.note.as_ref().unwrap().body, "hello");
        let updated = store.update_note("00000001", "world").unwrap();
        assert_eq!(updated.note.as_ref().unwrap().body, "world");
        let cleared = store.delete_note("00000001").unwrap();
        assert!(cleared.note.is_none());
    }

    #[test]
    fn empty_note_rejected_and_missing_note_errors() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("00000001"), false, TaskStatus::Ready).unwrap();
        assert_eq!(store.add_note("00000001", "   ").unwrap_err(), StoreError::InvalidNote);
        assert_eq!(store.delete_note("00000001").unwrap_err(), StoreError::NoteNotFound("00000001".into()));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p pond-core store`
Expected: PASS.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/store.rs
git commit -m "feat(core): add note operations"
```

---

## Task 19: Merge & split

**Files:**
- Modify: `crates/pond-core/src/store.rs`

Mirrors `mergeItem` and `splitItem`. Merge appends the source title into a previous draft/ready note-less item and removes the source. Split keeps the first title in place (draft→ready) and inserts a second item below carrying the source's note and status.

- [ ] **Step 1: Write the failing test** — add to `store.rs` (new `impl` block):

```rust
impl TaskStore {
    pub fn merge_item(&self, id: &str, into_previous: &str, title: &str) -> Result<Option<TaskItem>> {
        self.with_file(true, |file| {
            let source = resolve_index(id, &file.items)?;
            let previous = resolve_index(into_previous, &file.items)?;
            let prev_ok = matches!(file.items[previous].status, TaskStatus::Draft | TaskStatus::Ready)
                && file.items[previous].note.is_none();
            if source == previous || !prev_ok {
                return Ok(None);
            }
            let now = Utc::now();
            let source_note = file.items[source].note.clone();
            file.items[previous].title.push_str(title);
            if source_note.is_some() {
                file.items[previous].note = source_note;
            }
            Self::mark_updated(file, previous, now);
            let merged = file.items[previous].clone();
            file.items.remove(source);
            Ok(Some(merged))
        })
    }

    pub fn split_item(
        &self,
        id: &str,
        first_title: &str,
        second_title: &str,
        requested_second_id: Option<&str>,
    ) -> Result<TaskItem> {
        let first = first_title.trim().to_string();
        let second = second_title.trim().to_string();
        if first.is_empty() || second.is_empty() {
            return Err(StoreError::InvalidTitle);
        }
        self.with_file(true, |file| {
            let source = resolve_index(id, &file.items)?;
            let existing: HashSet<String> = file.items.iter().map(|i| i.id.clone()).collect();
            let second_id = match requested_second_id {
                Some(id) => id.to_string(),
                None => make_id(&existing),
            };
            if !is_valid_id(&second_id) {
                return Err(StoreError::InvalidId(second_id));
            }
            if existing.contains(&second_id) {
                return Err(StoreError::DuplicateId(second_id));
            }
            let now = Utc::now();
            let source_item = file.items[source].clone();
            file.items[source].title = first;
            file.items[source].note = None;
            if file.items[source].status == TaskStatus::Draft {
                file.items[source].status = TaskStatus::Ready;
            }
            Self::mark_updated(file, source, now);

            let mut second_item = TaskItem::new(second_id, second, source_item.collection.clone(), source_item.status, now);
            second_item.version = make_version(&file.items.iter().map(|i| i.version.clone()).collect());
            second_item.note = source_item.note;
            file.items.insert(source + 1, second_item.clone());
            Ok(second_item)
        })
    }
}
```

Add to `mod tests`:

```rust
    #[test]
    fn merge_appends_into_previous_draft() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("Hello ", "Inbox", Some("00000001"), false, TaskStatus::Draft).unwrap();
        store.add("world", "Inbox", Some("00000002"), false, TaskStatus::Ready).unwrap();
        let merged = store.merge_item("00000002", "00000001", "world").unwrap().unwrap();
        assert_eq!(merged.title, "Hello world");
        assert_eq!(store.items(None, None, &[], None).unwrap().len(), 1);
    }

    #[test]
    fn merge_refuses_non_draft_previous() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Inbox", Some("00000001"), false, TaskStatus::Completed).unwrap();
        store.add("b", "Inbox", Some("00000002"), false, TaskStatus::Ready).unwrap();
        assert!(store.merge_item("00000002", "00000001", "b").unwrap().is_none());
    }

    #[test]
    fn split_keeps_first_and_inserts_second() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("HelloWorld", "Inbox", Some("00000001"), false, TaskStatus::Draft).unwrap();
        let second = store.split_item("00000001", "Hello", "World", Some("00000002")).unwrap();
        assert_eq!(second.title, "World");
        let items = store.items(None, None, &[], None).unwrap();
        assert_eq!(items[0].title, "Hello");
        assert_eq!(items[0].status, TaskStatus::Ready); // draft promoted
        assert_eq!(items[1].title, "World");
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p pond-core store`
Expected: PASS.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/store.rs
git commit -m "feat(core): add merge and split operations"
```

---

## Task 20: Collection CRUD

**Files:**
- Modify: `crates/pond-core/src/store.rs`

Mirrors `createCollection`, `renameCollection`, `moveCollection`, `setCollectionColor/Archived/Prompt`, `deleteEmptyCollection`, `deleteCollection`, plus the `renameCollectionReference` helper. The default collection `Inbox` cannot be renamed/moved/deleted.

- [ ] **Step 1: Write the failing test** — add to `store.rs` (new `impl` block):

```rust
use crate::collections::{
    collection_api_name, collection_display_name, collection_group_name_for_api, collection_summary,
    normalized_collection_list, normalized_explicit_collection, normalized_explicit_group,
    remove_collection_from_groups, add_collection_group_if_missing, normalize_groups_in_file,
    collection_exists, DEFAULT_COLLECTION,
};
use crate::model::{CollectionColor, CollectionSummary};

impl TaskStore {
    fn rename_collection_reference(file: &mut TaskFile, old: &str, new: &str) -> Result<()> {
        let old = normalized_explicit_collection(old)?;
        let new = normalized_explicit_collection(new)?;
        let old_color = file.collection_colors.remove(&old);
        let old_prompt = file.collection_prompts.remove(&old);
        let was_archived = file.archived_collections.remove(&old);
        let new_group = collection_group_name_for_api(&new);
        for item in file.items.iter_mut().filter(|i| i.collection == old) {
            item.collection = new.clone();
        }
        file.collections.retain(|c| c != &old);
        file.collections = normalized_collection_list(file.collections.iter().cloned().chain([new.clone()]).collect::<Vec<_>>());
        remove_collection_from_groups(&old, file);
        add_collection_group_if_missing(&new_group, file);
        if let Some(group) = file.collection_groups.iter_mut().find(|g| g.name == new_group) {
            group.collections = normalized_collection_list(group.collections.iter().cloned().chain([new.clone()]).collect::<Vec<_>>());
        }
        file.collection_colors.entry(new.clone()).or_insert(old_color.unwrap_or(CollectionColor::Gray));
        if let Some(prompt) = old_prompt {
            file.collection_prompts.entry(new.clone()).or_insert(prompt);
        }
        if was_archived {
            file.archived_collections.insert(new);
        }
        normalize_groups_in_file(file);
        Ok(())
    }

    pub fn create_collection(&self, name: &str, group: &str) -> Result<String> {
        let reference = crate::collections::parse_reference(name, group)?;
        let api = reference.api_name();
        self.with_file(true, |file| {
            add_collection_if_missing(&api, Some(&reference.group_name), file)?;
            Ok(api.clone())
        })
    }

    pub fn rename_collection(&self, old: &str, new: &str) -> Result<String> {
        let clean_old = normalized_explicit_collection(old)?;
        if clean_old == DEFAULT_COLLECTION {
            return Err(StoreError::DefaultCollection);
        }
        if new.trim().is_empty() {
            return Err(StoreError::InvalidCollection);
        }
        self.with_file(true, |file| {
            if !collection_exists(&clean_old, file) {
                return Err(StoreError::CollectionNotFound(clean_old.clone()));
            }
            let old_group = collection_group_containing(&clean_old, file)
                .unwrap_or_else(|| collection_group_name_for_api(&clean_old));
            let reference = crate::collections::parse_reference(new, &old_group)?;
            let new_name = reference.api_name();
            if clean_old == new_name {
                return Ok(new_name);
            }
            if collection_exists(&new_name, file) {
                return Err(StoreError::CollectionConflict(new_name));
            }
            Self::rename_collection_reference(file, &clean_old, &new_name)?;
            Ok(new_name)
        })
    }

    pub fn move_collection(&self, name: &str, to_group: &str) -> Result<CollectionSummary> {
        let clean = normalized_explicit_collection(name)?;
        let group = normalized_explicit_group(to_group)?;
        if clean == DEFAULT_COLLECTION {
            return Err(StoreError::DefaultCollection);
        }
        self.with_file(true, |file| {
            if !collection_exists(&clean, file) {
                return Err(StoreError::CollectionNotFound(clean.clone()));
            }
            let new_name = collection_api_name(&group, &collection_display_name(&clean));
            if clean != new_name {
                if collection_exists(&new_name, file) {
                    return Err(StoreError::CollectionConflict(new_name));
                }
                Self::rename_collection_reference(file, &clean, &new_name)?;
            } else {
                move_collection_in_file(&clean, &group, file);
            }
            Ok(collection_summary(&new_name, file))
        })
    }

    pub fn set_collection_color(&self, name: &str, color: CollectionColor) -> Result<CollectionSummary> {
        let clean = normalized_explicit_collection(name)?;
        self.with_file(true, |file| {
            if !collection_exists(&clean, file) {
                return Err(StoreError::CollectionNotFound(clean.clone()));
            }
            add_collection_if_missing(&clean, None, file)?;
            file.collection_colors.insert(clean.clone(), color);
            Ok(collection_summary(&clean, file))
        })
    }

    pub fn set_collection_archived(&self, name: &str, is_archived: bool) -> Result<CollectionSummary> {
        let clean = normalized_explicit_collection(name)?;
        self.with_file(true, |file| {
            if !collection_exists(&clean, file) {
                return Err(StoreError::CollectionNotFound(clean.clone()));
            }
            add_collection_if_missing(&clean, None, file)?;
            if is_archived {
                file.archived_collections.insert(clean.clone());
            } else {
                file.archived_collections.remove(&clean);
            }
            Ok(collection_summary(&clean, file))
        })
    }

    pub fn set_collection_prompt(&self, name: &str, prompt: Option<&str>) -> Result<CollectionSummary> {
        let clean = normalized_explicit_collection(name)?;
        let clean_prompt = prompt.map(str::trim).filter(|p| !p.is_empty()).map(str::to_string);
        self.with_file(true, |file| {
            if !collection_exists(&clean, file) {
                return Err(StoreError::CollectionNotFound(clean.clone()));
            }
            add_collection_if_missing(&clean, None, file)?;
            match clean_prompt {
                Some(p) => { file.collection_prompts.insert(clean.clone(), p); }
                None => { file.collection_prompts.remove(&clean); }
            }
            Ok(collection_summary(&clean, file))
        })
    }

    pub fn delete_empty_collection(&self, name: &str) -> Result<bool> {
        let clean = normalized_explicit_collection(name)?;
        if clean == DEFAULT_COLLECTION {
            return Err(StoreError::DefaultCollection);
        }
        self.with_file(true, |file| {
            if file.items.iter().any(|i| i.collection == clean) {
                return Ok(false);
            }
            let before = file.collections.len();
            file.collections.retain(|c| c != &clean);
            file.collection_colors.remove(&clean);
            file.collection_prompts.remove(&clean);
            file.archived_collections.remove(&clean);
            remove_collection_from_groups(&clean, file);
            Ok(file.collections.len() != before)
        })
    }

    pub fn delete_collection(&self, name: &str) -> Result<bool> {
        let clean = normalized_explicit_collection(name)?;
        if clean == DEFAULT_COLLECTION {
            return Err(StoreError::DefaultCollection);
        }
        self.with_file(true, |file| {
            if !collection_exists(&clean, file) {
                return Err(StoreError::CollectionNotFound(clean.clone()));
            }
            let before_c = file.collections.len();
            let before_i = file.items.len();
            file.collections.retain(|c| c != &clean);
            file.collection_colors.remove(&clean);
            file.collection_prompts.remove(&clean);
            file.archived_collections.remove(&clean);
            remove_collection_from_groups(&clean, file);
            file.items.retain(|i| i.collection != clean);
            Ok(file.collections.len() != before_c || file.items.len() != before_i)
        })
    }
}
```

Add to `mod tests`:

```rust
    #[test]
    fn create_rename_and_protect_default() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let created = store.create_collection("Errands", crate::collections::DEFAULT_GROUP).unwrap();
        assert_eq!(created, "Errands");
        let renamed = store.rename_collection("Errands", "Personal").unwrap();
        assert_eq!(renamed, "Personal");
        assert_eq!(store.rename_collection("Inbox", "Nope").unwrap_err(), StoreError::DefaultCollection);
    }

    #[test]
    fn color_archive_prompt_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.create_collection("Work/A", crate::collections::DEFAULT_GROUP).unwrap();
        assert_eq!(store.set_collection_color("Work/A", CollectionColor::Blue).unwrap().color, CollectionColor::Blue);
        assert!(store.set_collection_archived("Work/A", true).unwrap().is_archived);
        assert_eq!(store.set_collection_prompt("Work/A", Some("hi")).unwrap().prompt_template.as_deref(), Some("hi"));
    }

    #[test]
    fn delete_collection_removes_items() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.add("a", "Work/A", Some("00000001"), false, TaskStatus::Ready).unwrap();
        assert!(store.delete_collection("Work/A").unwrap());
        assert!(store.items(None, None, &[], None).unwrap().is_empty());
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p pond-core store`
Expected: PASS.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/store.rs
git commit -m "feat(core): add collection CRUD operations"
```

---

## Task 21: Group CRUD

**Files:**
- Modify: `crates/pond-core/src/store.rs`

Mirrors `createCollectionGroup`, `renameCollectionGroup`, `deleteCollectionGroup`. Renaming/deleting a group relocates its collections (delete moves them to the default group). The default group is protected. (Reorder/merge are intentionally excluded — merge is composed in the command layer in a later phase.)

- [ ] **Step 1: Write the failing test** — add to `store.rs` (new `impl` block):

```rust
use crate::collections::DEFAULT_GROUP;

impl TaskStore {
    pub fn create_group(&self, name: &str) -> Result<String> {
        let clean = normalized_explicit_group(name)?;
        self.with_file(true, |file| {
            add_collection_group_if_missing(&clean, file);
            Ok(clean.clone())
        })
    }

    pub fn rename_group(&self, old: &str, new: &str) -> Result<String> {
        let clean_old = normalized_explicit_group(old)?;
        let clean_new = normalized_explicit_group(new)?;
        if clean_old == DEFAULT_GROUP {
            return Err(StoreError::DefaultCollectionGroup);
        }
        self.with_file(true, |file| {
            normalize_groups_in_file(file);
            let moved: Vec<String> = match file.collection_groups.iter().find(|g| g.name == clean_old) {
                Some(g) => g.collections.clone(),
                None => return Err(StoreError::CollectionGroupNotFound(clean_old.clone())),
            };
            if clean_old == clean_new {
                return Ok(clean_new.clone());
            }
            for collection in &moved {
                let target = collection_api_name(&clean_new, &collection_display_name(collection));
                Self::rename_collection_reference(file, collection, &target)?;
            }
            file.collection_groups.retain(|g| g.name != clean_old);
            add_collection_group_if_missing(&clean_new, file);
            normalize_groups_in_file(file);
            Ok(clean_new.clone())
        })
    }

    pub fn delete_group(&self, name: &str) -> Result<bool> {
        let clean = normalized_explicit_group(name)?;
        if clean == DEFAULT_GROUP {
            return Err(StoreError::DefaultCollectionGroup);
        }
        self.with_file(true, |file| {
            normalize_groups_in_file(file);
            let collections: Vec<String> = match file.collection_groups.iter().find(|g| g.name == clean) {
                Some(g) => g.collections.clone(),
                None => return Err(StoreError::CollectionGroupNotFound(clean.clone())),
            };
            file.collection_groups.retain(|g| g.name != clean);
            for collection in &collections {
                let target = collection_api_name(DEFAULT_GROUP, &collection_display_name(collection));
                Self::rename_collection_reference(file, collection, &target)?;
            }
            normalize_groups_in_file(file);
            Ok(true)
        })
    }
}
```

Add to `mod tests`:

```rust
    #[test]
    fn create_and_rename_group_moves_collections() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.create_collection("Work/A", crate::collections::DEFAULT_GROUP).unwrap();
        store.rename_group("Work", "Office").unwrap();
        let groups = store.collection_group_summaries().unwrap();
        assert!(groups.iter().any(|g| g.name == "Office"));
        assert!(!groups.iter().any(|g| g.name == "Work"));
    }

    #[test]
    fn delete_group_moves_collections_to_default() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.create_collection("Work/A", crate::collections::DEFAULT_GROUP).unwrap();
        assert!(store.delete_group("Work").unwrap());
        // "A" should now be a bare default-group collection.
        assert!(store.collection_summaries().unwrap().iter().any(|c| c.name == "A"));
    }

    #[test]
    fn default_group_is_protected() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        assert_eq!(store.rename_group(crate::collections::DEFAULT_GROUP, "X").unwrap_err(), StoreError::DefaultCollectionGroup);
        assert_eq!(store.delete_group(crate::collections::DEFAULT_GROUP).unwrap_err(), StoreError::DefaultCollectionGroup);
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p pond-core store`
Expected: PASS.

- [ ] **Step 3: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/store.rs
git commit -m "feat(core): add collection group CRUD operations"
```

---

## Task 22: Prompt templates & export

**Files:**
- Create: `crates/pond-core/src/prompt.rs`
- Create: `crates/pond-core/src/export.rs`
- Modify: `crates/pond-core/src/lib.rs`

`prompt.rs` mirrors `TaskPromptTemplate` (`{{token}}` substitution; unknown tokens are left verbatim) and carries the built-in default template. `export.rs` mirrors `CollectionExportPayload`: pretty JSON or one-item-per-line JSONL.

- [ ] **Step 1: Write the failing test** — create `crates/pond-core/src/prompt.rs`

```rust
use std::collections::HashMap;

pub const APPLICATION_DEFAULT_TEMPLATE: &str = "Run `{{cliCommand}}` and complete the listed tasks. Use `taskpond item update [task id] --status [status]` to update task status. Skip `Draft` tasks. Mark unclear, unnatural, or clearly unrelated tasks as `on-hold`. Mark tasks as `in-progress` when started and `aborted` if they cannot be completed. Group related work into appropriate commits. Use sub-agents with separate worktrees when parallelization helps, then merge their branches into the current branch. Before finishing, run `{{cliCommand}}` again because the user may add more tasks, and ensure no uncommitted changes remain.";

/// Substitute `{{token}}` occurrences from `variables`; unknown tokens are kept verbatim.
pub fn evaluate(template: &str, variables: &HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut rest = template;
    while let Some(open) = rest.find("{{") {
        result.push_str(&rest[..open]);
        let after_open = &rest[open + 2..];
        match after_open.find("}}") {
            Some(close) => {
                let token = after_open[..close].trim();
                match variables.get(token) {
                    Some(value) => result.push_str(value),
                    None => {
                        result.push_str("{{");
                        result.push_str(&after_open[..close]);
                        result.push_str("}}");
                    }
                }
                rest = &after_open[close + 2..];
            }
            None => {
                result.push_str(&rest[open..]);
                return result;
            }
        }
    }
    result.push_str(rest);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_known_tokens() {
        let vars: HashMap<String, String> = [("cliCommand".to_string(), "taskpond item get -c X".to_string())].into_iter().collect();
        let out = evaluate("Run `{{cliCommand}}` now", &vars);
        assert_eq!(out, "Run `taskpond item get -c X` now");
    }

    #[test]
    fn keeps_unknown_tokens_verbatim() {
        let out = evaluate("a {{missing}} b", &HashMap::new());
        assert_eq!(out, "a {{missing}} b");
    }

    #[test]
    fn default_template_mentions_cli_command() {
        assert!(APPLICATION_DEFAULT_TEMPLATE.contains("{{cliCommand}}"));
    }
}
```

- [ ] **Step 2: Create `crates/pond-core/src/export.rs`**

```rust
use crate::error::Result;
use crate::json::{to_compact_sorted, to_pretty_sorted};
use crate::model::TaskItem;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Jsonl,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayload {
    pub collection: String,
    pub exported_at: DateTime<Utc>,
    pub items: Vec<TaskItem>,
}

impl ExportPayload {
    pub fn encode(&self, format: ExportFormat) -> Result<String> {
        match format {
            ExportFormat::Json => to_pretty_sorted(self),
            ExportFormat::Jsonl => {
                if self.items.is_empty() {
                    return Ok(String::new());
                }
                let mut lines = Vec::with_capacity(self.items.len());
                for item in &self.items {
                    lines.push(to_compact_sorted(item)?);
                }
                Ok(format!("{}\n", lines.join("\n")))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn payload() -> ExportPayload {
        let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();
        let mut item = TaskItem::new("00000001".into(), "t".into(), "Inbox".into(), crate::model::TaskStatus::Ready, now);
        item.version = "v".repeat(12);
        ExportPayload { collection: "Inbox".into(), exported_at: now, items: vec![item] }
    }

    #[test]
    fn json_is_pretty_with_wrapper() {
        let out = payload().encode(ExportFormat::Json).unwrap();
        assert!(out.contains("\"collection\""));
        assert!(out.contains("\"exportedAt\""));
        assert!(out.contains("\"items\""));
        assert!(out.starts_with("{\n"));
    }

    #[test]
    fn jsonl_is_one_item_per_line_trailing_newline() {
        let out = payload().encode(ExportFormat::Jsonl).unwrap();
        assert!(out.ends_with('\n'));
        assert_eq!(out.lines().count(), 1);
        assert!(!out.contains("exportedAt"), "jsonl emits raw items, not the wrapper");
    }

    #[test]
    fn empty_jsonl_is_empty_string() {
        let mut p = payload();
        p.items.clear();
        assert_eq!(p.encode(ExportFormat::Jsonl).unwrap(), "");
    }
}
```

- [ ] **Step 3: Wire the modules** — append to `lib.rs`:

```rust
pub mod export;
pub mod prompt;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p pond-core`
Expected: PASS (entire suite green).

- [ ] **Step 5: Lint, format, commit**

```bash
cargo fmt && cargo clippy -p pond-core -- -D warnings
git add crates/pond-core/src/prompt.rs crates/pond-core/src/export.rs crates/pond-core/src/lib.rs
git commit -m "feat(core): add prompt template evaluation and collection export"
```

---

## Task 23: Public API surface & full-suite gate

**Files:**
- Modify: `crates/pond-core/src/lib.rs`

Finalize the crate's public surface so Phase 2 (CLI) and Phase 3 (Tauri) consume one clean namespace, and confirm the whole suite passes.

- [ ] **Step 1: Set `crates/pond-core/src/lib.rs` to the final wiring**

```rust
//! Pond core: domain model and file-backed task store.
//!
//! Pure, UI-agnostic library shared by the Tauri app and the `taskpond` CLI.

pub mod collections;
pub mod document;
pub mod error;
pub mod export;
pub mod ids;
pub mod json;
pub mod model;
pub mod paths;
pub mod prompt;
pub mod store;

pub use collections::{DEFAULT_COLLECTION, DEFAULT_GROUP};
pub use document::{TaskCollectionGroup, TaskFile};
pub use error::{Result, StoreError};
pub use export::{ExportFormat, ExportPayload};
pub use model::{
    CollectionColor, CollectionGroupSummary, CollectionSummary, TaskItem, TaskNote, TaskStatus,
};
pub use store::TaskStore;
```

- [ ] **Step 2: Add a public-API smoke test** — append to `crates/pond-core/src/lib.rs`:

```rust
#[cfg(test)]
mod api_smoke {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn end_to_end_via_public_api() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let item = store.add("Write spec", "Work/Docs", None, false, TaskStatus::Ready).unwrap();
        store.add_note(&item.id, "outline first").unwrap();
        let summaries = store.collection_summaries().unwrap();
        assert!(summaries.iter().any(|c: &CollectionSummary| c.name == "Work/Docs"));
        let groups: Vec<CollectionGroupSummary> = store.collection_group_summaries().unwrap();
        assert!(groups.iter().any(|g| g.name == "Work"));
    }
}
```

- [ ] **Step 3: Run the full suite**

Run: `cargo test -p pond-core`
Expected: PASS (all tests across all modules).

- [ ] **Step 4: Lint + format the whole crate**

Run: `cargo fmt && cargo clippy -p pond-core -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/pond-core/src/lib.rs
git commit -m "feat(core): finalize pond-core public API"
```

---

## Self-Review (completed during planning)

**Spec coverage (spec §4, §7, §8 — the `pond-core` parts):**
- Domain types (TaskItem/Note/Status/Color/summaries) → Tasks 3, 5, 6. ✅
- ID/version generation → Task 4. ✅
- On-disk store, fresh `version: 1`, camelCase, sorted keys → Tasks 7, 8. ✅
- File lock + atomic write → Task 10. ✅
- Cross-platform path + `POND_STORE` → Task 9. ✅
- Default collection `Inbox` / sentinel group "DefaultGroup" → Tasks 11–13. ✅
- Items query/filters/prefix-id → Task 14. ✅
- add/update/move/delete/clear/status/notes/merge/split → Tasks 15–19. ✅
- Collection & group CRUD → Tasks 20, 21. ✅
- Prompt-template eval + export encoding → Task 22. ✅
- Public API for downstream phases → Task 23. ✅
- **Intentionally excluded (out of scope this phase):** reorder operations (deferred app-wide); the macOS CLI installer (Phase 2); group *merge* (composed in the command layer in a later phase). These are noted, not gaps.

**Placeholder scan:** No TBD/TODO/"handle edge cases"; every code step shows complete code; every test step shows real assertions. ✅

**Type consistency:** Method names used by tests match the implementations defined in the same task (`add`, `update`/`update_if_current`, `move_item`, `delete`/`delete_many`/`delete_if_current`, `set_status`/`set_statuses`/`set_status_if_current`, `add_note`/`update_note`/`delete_note`, `merge_item`, `split_item`, `create_collection`/`rename_collection`/`move_collection`/`delete_collection`/`delete_empty_collection`, `set_collection_color`/`_archived`/`_prompt`, `create_group`/`rename_group`/`delete_group`). Shared helpers (`with_file`, `resolve_index`, `mark_updated`, `apply_update`, collection/group helpers) are defined before use. `TaskFile` field names are consistent across Tasks 8–21. ✅
