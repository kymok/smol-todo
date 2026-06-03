# Phase 5B: Prompts + Export + Clipboard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the "get data out" cluster from the approved 5B design: per-collection prompt overrides (an editor + Copy Prompt / Copy CLI Command), per-collection export to JSON/JSONL via a save dialog, clipboard (Copy ID / Copy Prompt / Copy CLI Command), and the Settings **Prompt** tab (edits the app-default template). Adds the Tauri **clipboard-manager** and **dialog** plugins (Rust crates + npm packages + capability permissions). Almost all logic already lives in `pond-core` (`prompt::evaluate` + `APPLICATION_DEFAULT_TEMPLATE`, `ExportPayload.encode`, `set_collection_prompt`); 5B is the IPC + UI wiring plus a small pure `prompt.rs` (3-level precedence + `shell_escape` + `cli_command`).

**Architecture:** `pond-core` stays the single source of truth; prompt **resolution** and **export encoding** happen server-side in `pond-tauri` because they need both the `TaskStore` and the `Mutex<Settings>` (introduced in 5A). The established seam holds: pure/testable functions take plain inputs (`prompt.rs`) or `&TaskStore` (`mutations.rs`); thin `#[tauri::command]` wrappers in `commands.rs` do `State` access + `Result<_, String>` mapping and are registered in `main.rs`. `collection_prompt_text` reads the collection's raw `promptTemplate` from `collection_summaries()` + `settings.default_prompt_template`, applies the 3-level precedence (`prompt::effective_collection_template`), then `pond_core::prompt::evaluate` with the two variables `cliCommand` + `collectionName`. `export_text` builds an `ExportPayload { collection, exported_at: Utc::now(), items }` and `encode`s it; `export_collection` writes the encoded string atomically (temp+rename). The frontend keeps its single `invoke` site (`api/client.ts`) plus a thin `lib/clipboard.ts` (the clipboard plugin's `writeText`); the export save picker uses the dialog plugin's `save` directly in `Sidebar.tsx`. The collection's raw override seeds `PromptEditorDialog` straight from the snapshot's `CollectionSummary.promptTemplate` — no read command needed. UI errors route through the existing `onError` from 5A.

**Tech Stack:** Rust 1.96.0 (pinned), Tauri v2 (`tauri` 2.11.2), `tauri-plugin-clipboard-manager` v2 + `tauri-plugin-dialog` v2, `serde`/`serde_json`, `chrono` 0.4 (added to `pond-tauri`, matching `pond-core`), `tempfile` (dev); Vite + React 18 + TypeScript + `@radix-ui/themes` 3.3.0 + `@radix-ui/react-icons`; `@tauri-apps/api` v2 (`core`); `@tauri-apps/plugin-clipboard-manager` + `@tauri-apps/plugin-dialog`; Vitest. npm.

---

## Conventions (read this section before `## File Structure`)

Every task obeys these. They are not repeated per step.

- **Branch:** work on the existing `tauri-radix-migration` branch. Do **not** create a new branch and do **not** set an upstream.
- **Rust toolchain:** pinned `1.96.0` (already in `rust-toolchain.toml`). Run all `cargo`/`npm`/`npx` commands from the repo root (the Vite root is the repo root).
- **Per Rust task gate:** `cargo fmt --all` then `cargo clippy --workspace --all-targets -- -D warnings` must be clean, and `cargo test -p pond-tauri` green.
- **Per frontend task gate:** `npx tsc --noEmit` clean, `npm run build` succeeds (it runs `tsc --noEmit && vite build`), `npx vitest run` green.
- **Imports/`use` at the top:** ALL `import` (TS) and `use` (Rust) statements live at the top of the file. In Rust test modules, all `use` go at the top of `mod tests` (i.e. directly under `#[cfg(test)] mod tests {`). No mid-file imports.
- **Radix Themes defaults only:** stock Radix parts, built-in named palette, no theme customization. The TSX below targets the installed `@radix-ui/themes` **3.3.0** API. Where a sample's component/prop shape differs from what 3.3.0 actually exports (e.g. `Dialog`, `TextArea`, `Tabs`, `ContextMenu.Sub`/`SubTrigger`/`SubContent`), **adjust the usage to the installed API** — the gate is a clean `tsc --noEmit` + `npm run build` (report any adjustment in the task's commit/notes). This is the established Phase 3–5A latitude, **not** a placeholder to leave logic unwritten.
- **No visual / DOM / screenshot tests.** Verification is logic unit tests (the `prompt.rs` Rust tests, the `mutations` export/prompt Rust tests, the `client.test.ts` + `clipboard.test.ts` mocked assertions) + the build/typecheck gates + manual `cargo tauri dev` launch (human check). Do **not** add `@testing-library`/`jsdom` render tests for the dialog, menus, save picker, or clipboard.
- **Command (invoke) names** must equal exactly what the frontend `client.ts` wrapper passes to `invoke` (`set_collection_prompt`, `collection_prompt_text`, `collection_cli_command`, `export_collection`).
- **Commit trailer:** every commit message ends with a trailing line:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`

### Verified facts (source of truth — do not guess)

- **`chrono` is NOT a `pond-tauri` dependency yet.** It is a `pond-core` dep (`chrono = { version = "0.4", features = ["serde"] }`, see `crates/pond-core/Cargo.toml`). Task 4 **adds** `chrono = { version = "0.4", features = ["serde"] }` to `src-tauri/Cargo.toml` so `export_text` can call `chrono::Utc::now()` to build `ExportPayload.exported_at`. (Match the version + the `serde` feature so the workspace resolves a single `chrono`.)
- **Plugin crate versions:** `tauri` resolves to **2.11.2** in `Cargo.lock`; `tauri-build = "2"`. Add the two plugins on the same major line: `tauri-plugin-clipboard-manager = "2"` and `tauri-plugin-dialog = "2"` (latest published 2.x as of 2026-06: clipboard-manager 2.3.2, dialog on the matching 2.x line — `"2"` lets cargo pick the newest compatible). Neither is in `Cargo.lock` yet (`grep -c` returns 0).
- **Plugin npm packages:** `@tauri-apps/plugin-clipboard-manager` and `@tauri-apps/plugin-dialog`, both on the `^2.0.0` line to match `@tauri-apps/api` `^2.0.0`. Neither is in `package.json` yet.
- **Plugin builder init:** in `main.rs`, `.plugin(tauri_plugin_clipboard_manager::init())` and `.plugin(tauri_plugin_dialog::init())` are chained on `tauri::Builder::default()` before `.invoke_handler(...)`.
- **JS plugin APIs:** clipboard write = `import { writeText } from "@tauri-apps/plugin-clipboard-manager"` → `await writeText(text)`. Save picker = `import { save } from "@tauri-apps/plugin-dialog"` → `await save({ defaultPath, filters: [{ name, extensions }] })` returns `string | null` (null = user cancelled).
- **Capability permission identifiers:** clipboard write = **`clipboard-manager:allow-write-text`**; dialog save = **`dialog:allow-save`**. These are added to `src-tauri/capabilities/default.json`'s `permissions` array alongside the existing `core:default` + `core:window:allow-set-always-on-top`. **IMPORTANT — verify against the generated schema:** `src-tauri/gen/schemas/desktop-schema.json` currently does **not** list these identifiers (the plugins aren't installed yet; `grep write-text` / `grep allow-save` return nothing). The schema is regenerated by `tauri-build` when the app is built **after** the plugin crates are in `Cargo.toml`. So in Task 1: add the deps + the builder init **first**, then `cargo build -p pond-tauri` (regenerates the schema), then `grep -o 'clipboard-manager:allow-write-text' src-tauri/gen/schemas/desktop-schema.json` and `grep -o 'dialog:allow-save' src-tauri/gen/schemas/desktop-schema.json` to **confirm** the identifiers exist; if either differs (e.g. a `:default` set is preferred), adjust `default.json` to the schema's spelling. Gate = clean build + the calls not rejected at manual launch.
- **`set_collection_prompt` (pond-core):** `TaskStore::set_collection_prompt(&self, name: &str, prompt: Option<&str>) -> Result<CollectionSummary>` (`crates/pond-core/src/store.rs:824`). It already trims + drops empty (`prompt.map(str::trim).filter(|p| !p.is_empty())`) and `remove`s the override when `None`/empty, and errors `CollectionNotFound` if the collection does not exist. The mutation wrapper calls it then rebuilds the full snapshot (it returns a single `CollectionSummary`, which we discard in favor of `build_snapshot`).
- **`items` (pond-core):** `TaskStore::items(&self, status: Option<TaskStatus>, collection: Option<&str>, ids: &[String], search: Option<&str>) -> Result<Vec<TaskItem>>` (`crates/pond-core/src/store.rs:126`). For a collection's items: `store.items(None, Some(name), &[], None)?`.
- **`collection_summaries` (pond-core):** `TaskStore::collection_summaries(&self) -> Result<Vec<CollectionSummary>>` (`crates/pond-core/src/store.rs:166`). `CollectionSummary { name: String, …, prompt_template: Option<String> }` (`crates/pond-core/src/model.rs:117`). The raw override is `summary.prompt_template` (an `Option<String>`); read it via `.iter().find(|c| c.name == name)`.
- **`prompt::evaluate` (pond-core):** `pub fn evaluate(template: &str, variables: &HashMap<String, String>) -> String` (`crates/pond-core/src/prompt.rs:6`); substitutes `{{token}}` (trimmed inside the braces), unknown tokens kept verbatim. `pub const APPLICATION_DEFAULT_TEMPLATE: &str = "Run `{{cliCommand}}` …"` (`crates/pond-core/src/prompt.rs:3`).
- **`ExportPayload` / `ExportFormat` (pond-core):** `pub enum ExportFormat { Json, Jsonl }`; `#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct ExportPayload { pub collection: String, pub exported_at: DateTime<Utc>, pub items: Vec<TaskItem> }`; `impl ExportPayload { pub fn encode(&self, format: ExportFormat) -> Result<String> }` — JSON = pretty (whole-object wrapper with keys `collection`/`exportedAt`/`items`), JSONL = one compact item per line with a trailing `\n`, **empty items → empty string for JSONL** (`crates/pond-core/src/export.rs:7-37`).
- **`SnapshotDto` (pond-tauri):** `mutations` functions return `Result<SnapshotDto>` via `build_snapshot(store)` (`src-tauri/src/commands.rs:9`, `src-tauri/src/mutations.rs`). `CollectionSummaryDto` already carries `prompt_template: Option<String>` and serializes camelCase as `promptTemplate` (`src-tauri/src/dto.rs:17,31`), so the frontend `CollectionSummary.promptTemplate?: string` (`src/api/types.ts:14`) already exposes the raw override — **no new read command for seeding the editor.**
- **Swift `shellEscaped` (EXACT — port precisely):** `Sources/PondApp/TaskViewSupport.swift:246`:
  ```swift
  extension String {
      var shellEscaped: String {
          "'\(replacingOccurrences(of: "'", with: "'\\''"))'"
      }
  }
  ```
  i.e. **replace every single-quote `'` with the four-character sequence `'\''`, then wrap the whole result in single quotes.** Rust port: `format!("'{}'", name.replace('\'', "'\\''"))`. (`name.replace('\'', "'\\''")` — the Rust string literal `"'\\''"` is the 4 chars `'`, `\`, `'`, `'`.)
- **Swift `cliCommand` (EXACT):** `Sources/PondApp/CollectionMenus.swift:161`: `"taskpond item get --collection \(collection.name.shellEscaped)"`. Rust: `format!("taskpond item get --collection {}", shell_escape(name))`.
- **Swift prompt precedence (EXACT):** `CollectionMenus.swift:173` `effectivePromptTemplate` returns `collection.promptTemplate` unless it is nil/whitespace-only-after-trim, in which case `TaskPromptSettings.effectiveDefaultPromptTemplate`; the default-setting layer returns the stored default unless empty-after-trim, else `APPLICATION_DEFAULT_TEMPLATE`. Variables (`TaskViewSupport.swift:736` `taskExamplePrompt`): exactly `{ "cliCommand": cliCommand, "collectionName": collectionName }`.
- **Swift Copy ID (EXACT):** `Sources/PondApp/TaskRow.swift:638` `copyIDToPasteboard` → `NSPasteboard.general.setString(item.id, …)`. Tauri: `copyText(item.id)`.
- **Swift export payload (EXACT):** `Sources/PondApp/CollectionExport.swift:123` `CollectionExportPayload { collection: String; exportedAt: Date; items: [TaskItem] }`, encoded as `json`/`jsonl` — matches pond-core's `ExportPayload` 1:1. Format enum `CollectionExportFormat { case json = "JSON"; case jsonl = "JSONL" }` (`CollectionExport.swift:6`).

### Divergences from the design spec (confirmed against source)

1. **Plugin versions pinned to the major line `"2"`** rather than an exact patch, matching the repo's existing `tauri = "2"` / `tauri-build = "2"` convention so cargo resolves a single compatible set against `tauri` 2.11.2.
2. **`chrono` added to `pond-tauri`** (Task 4): the design's `export_text` calls `Utc::now()`; since `chrono` is not yet a `pond-tauri` dep, Task 4 adds it (matching `pond-core`'s `0.4` + `serde` feature).
3. **`export_text` is non-deterministic on the timestamp** (`Utc::now()`), so its tests assert the **structural shape** (JSON wrapper keys `collection`/`exportedAt`/`items`; JSONL line count + trailing newline; empty-collection cases) rather than an exact string. The collection name + item titles are asserted exactly.
4. **`collection_prompt_text` evaluation is unit-tested indirectly:** the precedence logic is tested in `prompt.rs` (pure) and via a `collection_prompt_text`-style test against a tempdir store + a `Settings` value (Task 3); `evaluate` itself is already covered by pond-core's own tests.
5. **The editor seeds from the snapshot's `promptTemplate`** (already in `CollectionSummaryDto`) — no extra read command, per design §5.

---

## File Structure

```
src-tauri/
├─ Cargo.toml      + tauri-plugin-clipboard-manager = "2", tauri-plugin-dialog = "2", chrono = { version = "0.4", features = ["serde"] }
├─ capabilities/default.json  + clipboard-manager:allow-write-text + dialog:allow-save permissions
└─ src/
   ├─ prompt.rs    (NEW) pure: effective_default_template / effective_collection_template (3-level precedence, trim) + cli_command + shell_escape (Swift port) + #[cfg(test)] tests
   ├─ mutations.rs + set_collection_prompt(store,name,template)->SnapshotDto + export_text(store,name,format)->String
   ├─ commands.rs  + set_collection_prompt / collection_prompt_text / collection_cli_command / export_collection #[tauri::command] wrappers
   └─ main.rs      + mod prompt; .plugin(clipboard_manager::init()).plugin(dialog::init()); register the 4 commands

src/
├─ api/
│  ├─ client.ts       + setCollectionPrompt / collectionPromptText / collectionCliCommand / exportCollection wrappers
│  └─ client.test.ts  + mocked-invoke assertions for setCollectionPrompt + exportCollection
├─ lib/
│  ├─ clipboard.ts      (NEW, thin) copyText(text) via @tauri-apps/plugin-clipboard-manager writeText
│  └─ clipboard.test.ts (NEW) mocked-plugin assertion that copyText calls writeText
├─ components/
│  ├─ PromptEditorDialog.tsx (NEW) Radix Dialog + TextArea seeded from the collection's promptTemplate; Save → setCollectionPrompt
│  ├─ SettingsDialog.tsx     + Prompt tab (default-template editor) + settings/updateSettings props
│  ├─ Sidebar.tsx            collection menu: Edit Prompt…, Copy Prompt, Copy CLI Command, Export Collection ▸ As JSON/As JSONL
│  └─ TaskRow.tsx            item menu: Copy ID
└─ App.tsx         promptCollection state + render PromptEditorDialog; thread settings/updateSettings into SettingsDialog
```

Each unit stays focused: `prompt.rs` is pure (plain-string inputs, tested without Tauri); `mutations.rs` is `&TaskStore`-testable; `commands.rs` wrappers contain no logic beyond `State` access → string error; `api/`+`lib/` are the only `invoke`/plugin sites; components render and dispatch.

---

## Task 1: Plugins + capabilities + builder

Add the clipboard-manager + dialog plugin crates and npm packages, initialize them on the Tauri builder, and grant the two capability permissions. Foundation only — no unit tests; the gate is a clean Rust build (which also regenerates the capability schema so the permission identifiers can be confirmed) + a clean frontend build.

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, `src-tauri/capabilities/default.json`, `package.json`

- [ ] **Step 1: Add the plugin crates to `src-tauri/Cargo.toml`**

Edit `src-tauri/Cargo.toml`. In `[dependencies]`, after `directories = "5"`, add:

```toml
tauri-plugin-clipboard-manager = "2"
tauri-plugin-dialog = "2"
```

- [ ] **Step 2: Initialize the plugins on the builder in `main.rs`**

Edit `src-tauri/src/main.rs`. Chain the two `.plugin(...)` calls on `tauri::Builder::default()` immediately before `.invoke_handler(...)`:

```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
```

(Leave the rest of the builder chain unchanged.)

- [ ] **Step 3: Build the Rust crate (regenerates the capability schema) + confirm the permission identifiers**

```bash
cargo build -p pond-tauri
grep -o 'clipboard-manager:allow-write-text' src-tauri/gen/schemas/desktop-schema.json
grep -o 'dialog:allow-save' src-tauri/gen/schemas/desktop-schema.json
```
Expected: the build succeeds, and **both** greps print their identifier once. If either prints nothing, open `src-tauri/gen/schemas/desktop-schema.json` and search for `write-text` / `save` to find the exact spelling (or a `clipboard-manager:default` / `dialog:default` set that includes them) and use that in Step 4 instead.

- [ ] **Step 4: Grant the two permissions in `capabilities/default.json`**

Edit `src-tauri/capabilities/default.json`. Extend the `permissions` array with the two confirmed identifiers:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-set-always-on-top",
    "clipboard-manager:allow-write-text",
    "dialog:allow-save"
  ]
}
```

(Use whatever spelling Step 3 confirmed. Gate = the dev build starts and `writeText` / `save` are not rejected at manual launch.)

- [ ] **Step 5: Add the npm packages to `package.json`**

Edit `package.json`. In `"dependencies"`, after `"@tauri-apps/api": "^2.0.0",`, add:

```json
    "@tauri-apps/plugin-clipboard-manager": "^2.0.0",
    "@tauri-apps/plugin-dialog": "^2.0.0",
```

- [ ] **Step 6: Install + build gate**

```bash
npm install
cargo build -p pond-tauri
npm run build
```
Expected: `npm install` adds both plugin packages; the Rust build is clean; `npm run build` (`tsc --noEmit && vite build`) succeeds. (No new code imports the plugins yet — that lands in Tasks 5–7 — so the JS packages are present but unused; that is fine for the build.)

- [ ] **Step 7: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/main.rs src-tauri/capabilities/default.json package.json package-lock.json src-tauri/gen/schemas/desktop-schema.json Cargo.lock
git commit -m "feat(tauri): add clipboard-manager + dialog plugins and capabilities

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 2: `prompt.rs` pure helpers + tests

A new pure module mirroring Swift's `effectivePromptTemplate` / `effectiveDefaultPromptTemplate`, `cliCommand`, and `shellEscaped`. No Tauri/`Settings`/`TaskStore` deps — plain string inputs, fully unit-tested.

**Files:**
- Create: `src-tauri/src/prompt.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write `prompt.rs` with the implementation + failing tests**

Create `src-tauri/src/prompt.rs`:

```rust
use pond_core::prompt::APPLICATION_DEFAULT_TEMPLATE;

/// The effective app-default template: the stored default if it is non-empty
/// after trimming, else the built-in `APPLICATION_DEFAULT_TEMPLATE`.
/// Port of Swift `TaskPromptSettings.effectiveDefaultPromptTemplate`.
pub fn effective_default_template(stored_default: &str) -> &str {
    if stored_default.trim().is_empty() {
        APPLICATION_DEFAULT_TEMPLATE
    } else {
        stored_default
    }
}

/// The effective template for a collection: the collection's own override if it
/// is non-empty after trimming, else the effective app-default. Port of Swift
/// `CollectionMenus.effectivePromptTemplate`.
pub fn effective_collection_template(collection_template: Option<&str>, stored_default: &str) -> String {
    match collection_template {
        Some(t) if !t.trim().is_empty() => t.to_string(),
        _ => effective_default_template(stored_default).to_string(),
    }
}

/// The `taskpond` command that lists a collection's items. Port of Swift
/// `CollectionMenus.cliCommand`.
pub fn cli_command(name: &str) -> String {
    format!("taskpond item get --collection {}", shell_escape(name))
}

/// Single-quote a value for POSIX shells, escaping embedded single quotes as
/// `'\''`. EXACT port of Swift `String.shellEscaped`
/// (`"'\(replacingOccurrences(of: "'", with: "'\\''"))'"`).
pub fn shell_escape(name: &str) -> String {
    format!("'{}'", name.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pond_core::prompt::APPLICATION_DEFAULT_TEMPLATE;

    #[test]
    fn effective_default_uses_stored_when_present() {
        assert_eq!(effective_default_template("My default"), "My default");
    }

    #[test]
    fn effective_default_falls_back_when_empty_or_whitespace() {
        assert_eq!(effective_default_template(""), APPLICATION_DEFAULT_TEMPLATE);
        assert_eq!(effective_default_template("   \n\t "), APPLICATION_DEFAULT_TEMPLATE);
    }

    #[test]
    fn effective_collection_prefers_override() {
        assert_eq!(
            effective_collection_template(Some("Collection prompt"), "Stored default"),
            "Collection prompt"
        );
    }

    #[test]
    fn effective_collection_falls_back_to_stored_default() {
        // Override absent or whitespace-only → the stored default.
        assert_eq!(
            effective_collection_template(None, "Stored default"),
            "Stored default"
        );
        assert_eq!(
            effective_collection_template(Some("   "), "Stored default"),
            "Stored default"
        );
    }

    #[test]
    fn effective_collection_falls_back_to_builtin_when_both_empty() {
        assert_eq!(
            effective_collection_template(None, ""),
            APPLICATION_DEFAULT_TEMPLATE
        );
        assert_eq!(
            effective_collection_template(Some("  "), "  "),
            APPLICATION_DEFAULT_TEMPLATE
        );
    }

    #[test]
    fn shell_escape_plain_name() {
        assert_eq!(shell_escape("Work"), "'Work'");
    }

    #[test]
    fn shell_escape_name_with_spaces() {
        assert_eq!(shell_escape("Work Docs"), "'Work Docs'");
    }

    #[test]
    fn shell_escape_name_with_single_quote() {
        // A single quote becomes the 4-char sequence '\'' inside the wrapping quotes.
        assert_eq!(shell_escape("Bob's"), "'Bob'\\''s'");
    }

    #[test]
    fn cli_command_format() {
        assert_eq!(
            cli_command("Work"),
            "taskpond item get --collection 'Work'"
        );
        assert_eq!(
            cli_command("Bob's list"),
            "taskpond item get --collection 'Bob'\\''s list'"
        );
    }
}
```

- [ ] **Step 2: Declare the module in `main.rs`**

Edit `src-tauri/src/main.rs`. Add `mod prompt;` to the module list (keep it alphabetical, after `mod mutations;`):

```rust
mod commands;
mod dto;
mod mutations;
mod prompt;
mod settings;
mod watcher;
```

- [ ] **Step 3: Run the tests + gate**

```bash
cargo test -p pond-tauri prompt
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: all 9 `prompt::tests` pass; fmt/clippy clean. (If `prompt.rs` exists but `mod prompt;` is missing, the tests don't compile — declaring the module in Step 2 fixes that.)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/prompt.rs src-tauri/src/main.rs
git commit -m "feat(tauri): pure prompt helpers (precedence + cli_command + shell_escape)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 3: prompt/cli commands (`set_collection_prompt` + `collection_prompt_text` + `collection_cli_command`)

Add the prompt-override mutation and the two read commands. `set_collection_prompt` delegates to pond-core then rebuilds the snapshot; `collection_prompt_text` resolves the 3-level precedence + `evaluate` server-side; `collection_cli_command` is `prompt::cli_command`.

**Files:**
- Modify: `src-tauri/src/mutations.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Add `set_collection_prompt` to `mutations.rs` with a failing test**

Edit `src-tauri/src/mutations.rs`. Append the mutation after the other collection mutations (above any `#[cfg(test)]`):

```rust
/// Set or clear a collection's prompt override. `template` `None`/empty clears it
/// (pond-core trims + drops empty internally). Returns the rebuilt snapshot.
pub fn set_collection_prompt(
    store: &TaskStore,
    name: &str,
    template: Option<&str>,
) -> Result<SnapshotDto> {
    store.set_collection_prompt(name, template)?;
    build_snapshot(store)
}
```

Add a test module if `mutations.rs` does not already have one, or extend the existing `#[cfg(test)] mod tests`. The test module's top `use` block must include the items the test uses:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pond_core::TaskStatus;
    use tempfile::tempdir;

    #[test]
    fn set_collection_prompt_sets_and_clears_override() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add("Task", "Work", None, false, TaskStatus::Ready)
            .unwrap();

        // Set an override.
        let snap = set_collection_prompt(&store, "Work", Some("My prompt")).unwrap();
        let work = snap.collections.iter().find(|c| c.name == "Work").unwrap();
        assert_eq!(work.prompt_template.as_deref(), Some("My prompt"));

        // Clearing with None removes it.
        let snap = set_collection_prompt(&store, "Work", None).unwrap();
        let work = snap.collections.iter().find(|c| c.name == "Work").unwrap();
        assert_eq!(work.prompt_template, None);

        // Clearing with an empty/whitespace string also removes it (pond-core trims).
        set_collection_prompt(&store, "Work", Some("Set again")).unwrap();
        let snap = set_collection_prompt(&store, "Work", Some("   ")).unwrap();
        let work = snap.collections.iter().find(|c| c.name == "Work").unwrap();
        assert_eq!(work.prompt_template, None);
    }
}
```

(If a `mod tests` already exists in `mutations.rs`, merge the `use` lines into its existing top block and add only the `set_collection_prompt_sets_and_clears_override` fn — do not create a second `mod tests`.)

Run:

```bash
cargo test -p pond-tauri set_collection_prompt
```
Expected: **fail** to compile (the `mutations::set_collection_prompt` fn does not exist) → then, after Step 1's code is in, the test passes.

- [ ] **Step 2: Add the three command wrappers to `commands.rs` with a failing precedence test**

Edit `src-tauri/src/commands.rs`. Extend the top `use` block to bring in `prompt` + `HashMap`:

```rust
use crate::dto::{CollectionGroupSummaryDto, CollectionSummaryDto, SnapshotDto};
use crate::mutations;
use crate::prompt;
use crate::settings::{self, Settings};
use pond_core::{CollectionColor, Result, TaskItem, TaskStatus, TaskStore};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;
```

Append the three commands before the `#[cfg(test)]` module:

```rust
#[tauri::command]
pub fn set_collection_prompt(
    store: State<TaskStore>,
    name: String,
    template: Option<String>,
) -> std::result::Result<SnapshotDto, String> {
    mutations::set_collection_prompt(&store, &name, template.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn collection_prompt_text(
    store: State<TaskStore>,
    settings: State<Mutex<Settings>>,
    name: String,
) -> std::result::Result<String, String> {
    let summaries = store.collection_summaries().map_err(|e| e.to_string())?;
    let collection_template = summaries
        .iter()
        .find(|c| c.name == name)
        .and_then(|c| c.prompt_template.clone());
    let stored_default = {
        let guard = settings.lock().map_err(|e| e.to_string())?;
        guard.default_prompt_template.clone()
    };
    let template =
        prompt::effective_collection_template(collection_template.as_deref(), &stored_default);
    let variables = HashMap::from([
        ("cliCommand".to_string(), prompt::cli_command(&name)),
        ("collectionName".to_string(), name.clone()),
    ]);
    Ok(pond_core::prompt::evaluate(&template, &variables))
}

#[tauri::command]
pub fn collection_cli_command(name: String) -> String {
    prompt::cli_command(&name)
}
```

Extend the existing `#[cfg(test)] mod tests` at the bottom of `commands.rs`. Its top `use` block (under `#[cfg(test)] mod tests {`) must add `Settings` + `Mutex`:

```rust
    use super::*;
    use crate::settings::Settings;
    use pond_core::prompt::APPLICATION_DEFAULT_TEMPLATE;
    use pond_core::TaskStatus;
    use std::sync::Mutex;
    use tempfile::tempdir;
```

(Merge these with the file's existing `use super::*; use pond_core::TaskStatus; use tempfile::tempdir;` — do not duplicate lines. The commands take `State<…>`, which is awkward to construct in a unit test, so test the **resolution logic** directly via the same building blocks the command uses: `prompt::effective_collection_template` + `pond_core::prompt::evaluate`, fed from a real tempdir store + a `Settings` value. This is the same seam the command runs and exercises the precedence end-to-end without Tauri's `State`.)

Add this test fn inside `mod tests`:

```rust
    #[test]
    fn collection_prompt_text_precedence() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add("Task", "Work", None, false, TaskStatus::Ready)
            .unwrap();

        // Helper mirroring collection_prompt_text's resolution for a given settings default.
        let resolve = |default: &str| -> String {
            let summaries = store.collection_summaries().unwrap();
            let collection_template = summaries
                .iter()
                .find(|c| c.name == "Work")
                .and_then(|c| c.prompt_template.clone());
            let template = crate::prompt::effective_collection_template(
                collection_template.as_deref(),
                default,
            );
            let variables = std::collections::HashMap::from([
                ("cliCommand".to_string(), crate::prompt::cli_command("Work")),
                ("collectionName".to_string(), "Work".to_string()),
            ]);
            pond_core::prompt::evaluate(&template, &variables)
        };

        // No override, no default setting → built-in template, evaluated.
        let builtin = resolve("");
        assert!(builtin.contains("taskpond item get --collection 'Work'"));
        assert!(!builtin.contains("{{cliCommand}}"));
        // Sanity: the built-in source contains the token before evaluation.
        assert!(APPLICATION_DEFAULT_TEMPLATE.contains("{{cliCommand}}"));

        // Default setting layer (no collection override) wins over built-in.
        let from_default = resolve("Default: {{collectionName}} via {{cliCommand}}");
        assert_eq!(
            from_default,
            "Default: Work via taskpond item get --collection 'Work'"
        );

        // Collection override wins over the default setting.
        store
            .set_collection_prompt("Work", Some("Override: {{collectionName}}"))
            .unwrap();
        let from_override = resolve("Default: should be ignored");
        assert_eq!(from_override, "Override: Work");
    }
```

Run:

```bash
cargo test -p pond-tauri
```
Expected: the new tests compile + pass (they reference `crate::prompt::*` + `pond_core::prompt::evaluate`, all in place from Task 2).

- [ ] **Step 3: Register the three commands in `main.rs`**

Edit `src-tauri/src/main.rs`. In `generate_handler!`, append after `commands::store_path,`:

```rust
            commands::set_collection_prompt,
            commands::collection_prompt_text,
            commands::collection_cli_command,
```

- [ ] **Step 4: Test + gate**

```bash
cargo test -p pond-tauri
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: all `pond-tauri` tests pass (incl. `prompt::tests`, `mutations` set/clear, `commands` precedence); fmt/clippy clean.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/mutations.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(tauri): prompt override mutation + prompt-text/cli-command read commands

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 4: export (`export_text` mutation + `export_collection` command + `chrono`)

Add the export encoder (`export_text`) and the file-writing command (`export_collection`, atomic temp+rename), and add the `chrono` dependency for `Utc::now()`.

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/mutations.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Add `chrono` to `src-tauri/Cargo.toml`**

Edit `src-tauri/Cargo.toml`. In `[dependencies]`, after `tauri-plugin-dialog = "2"`, add (matching `pond-core`'s pin + feature):

```toml
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 2: Add `export_text` to `mutations.rs` with failing tests**

Edit `src-tauri/src/mutations.rs`. Extend the top `use` block to bring in the export types + `chrono`:

```rust
use crate::commands::build_snapshot;
use crate::dto::SnapshotDto;
use chrono::Utc;
use pond_core::export::{ExportFormat, ExportPayload};
use pond_core::{
    CollectionColor, Result, TaskItem, TaskStatus, TaskStore, DEFAULT_COLLECTION, DEFAULT_GROUP,
};
```

(If `pond_core::export` is not re-exported at the crate root, the path `pond_core::export::{ExportFormat, ExportPayload}` is correct — `export.rs` is a module of `pond-core`. Adjust only if `cargo build` reports a different path; gate = clean build.)

Append the encoder after `set_collection_prompt`:

```rust
/// Encode a collection's items as JSON or JSONL via pond-core's `ExportPayload`.
/// The timestamp is `Utc::now()`, so callers/tests must treat the output's time
/// as non-deterministic.
pub fn export_text(store: &TaskStore, name: &str, format: ExportFormat) -> Result<String> {
    let payload = ExportPayload {
        collection: name.to_string(),
        exported_at: Utc::now(),
        items: store.items(None, Some(name), &[], None)?,
    };
    payload.encode(format)
}
```

Add these tests to the `mutations` `mod tests` (merge `use` lines into the existing top block; add `use pond_core::export::ExportFormat;`):

```rust
    #[test]
    fn export_text_json_has_wrapper_keys() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add("Alpha", "Work", None, false, TaskStatus::Ready)
            .unwrap();
        store
            .add("Beta", "Work", None, false, TaskStatus::Ready)
            .unwrap();

        let out = export_text(&store, "Work", ExportFormat::Json).unwrap();
        // Pretty JSON wrapper (camelCase keys).
        assert!(out.contains("\"collection\""));
        assert!(out.contains("\"exportedAt\""));
        assert!(out.contains("\"items\""));
        assert!(out.contains("\"Work\""));
        assert!(out.contains("Alpha"));
        assert!(out.contains("Beta"));
    }

    #[test]
    fn export_text_jsonl_is_one_item_per_line() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add("Alpha", "Work", None, false, TaskStatus::Ready)
            .unwrap();
        store
            .add("Beta", "Work", None, false, TaskStatus::Ready)
            .unwrap();

        let out = export_text(&store, "Work", ExportFormat::Jsonl).unwrap();
        // Trailing newline; two content lines; no wrapper object.
        assert!(out.ends_with('\n'));
        let lines: Vec<&str> = out.trim_end().split('\n').collect();
        assert_eq!(lines.len(), 2);
        assert!(!out.contains("\"items\""));
        assert!(lines.iter().any(|l| l.contains("Alpha")));
        assert!(lines.iter().any(|l| l.contains("Beta")));
    }

    #[test]
    fn export_text_empty_collection() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        // No items added to "Empty"; create the collection so it exists.
        store.set_collection_color("Empty", CollectionColor::Gray).unwrap();

        // JSONL of an empty collection is the empty string (pond-core contract).
        let jsonl = export_text(&store, "Empty", ExportFormat::Jsonl).unwrap();
        assert_eq!(jsonl, "");

        // JSON still emits the wrapper with an empty items array.
        let json = export_text(&store, "Empty", ExportFormat::Json).unwrap();
        assert!(json.contains("\"items\""));
        assert!(json.contains("\"Empty\""));
    }
```

(`CollectionColor::Gray` is the variant name used elsewhere in `pond-tauri` tests; if `set_collection_color` is not the lightest way to materialize an empty collection in this codebase, use whichever existing store method creates a collection without items — gate = the test compiles + passes. The empty-collection items query simply returns `[]`.)

Run:

```bash
cargo test -p pond-tauri export_text
```
Expected: **fail** to compile (no `export_text`) → pass once Step 2's code is in.

- [ ] **Step 3: Add the `export_collection` command (atomic write) to `commands.rs`**

Edit `src-tauri/src/commands.rs`. Extend the top `use` block to bring in `ExportFormat`:

```rust
use crate::dto::{CollectionGroupSummaryDto, CollectionSummaryDto, SnapshotDto};
use crate::mutations;
use crate::prompt;
use crate::settings::{self, Settings};
use pond_core::export::ExportFormat;
use pond_core::{CollectionColor, Result, TaskItem, TaskStatus, TaskStore};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;
```

Append the command before `#[cfg(test)]`:

```rust
#[tauri::command]
pub fn export_collection(
    store: State<TaskStore>,
    name: String,
    format: String,
    path: String,
) -> std::result::Result<(), String> {
    let fmt = match format.as_str() {
        "json" => ExportFormat::Json,
        "jsonl" => ExportFormat::Jsonl,
        other => return Err(format!("unknown export format: {other}")),
    };
    let contents = mutations::export_text(&store, &name, fmt).map_err(|e| e.to_string())?;
    let target = std::path::Path::new(&path);
    let tmp = target.with_extension("tmp");
    std::fs::write(&tmp, contents.as_bytes()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, target).map_err(|e| e.to_string())?;
    Ok(())
}
```

(Atomic temp+rename mirrors `settings::save`. `with_extension("tmp")` replaces the chosen `.json`/`.jsonl` suffix on the temp file only; the final rename lands the user's chosen path.)

- [ ] **Step 4: Register `export_collection` in `main.rs`**

Edit `src-tauri/src/main.rs`. In `generate_handler!`, append after `commands::collection_cli_command,`:

```rust
            commands::export_collection,
```

- [ ] **Step 5: Test + gate**

```bash
cargo test -p pond-tauri
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: all tests pass (incl. the 3 `export_text` cases); fmt/clippy clean.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/mutations.rs src-tauri/src/commands.rs src-tauri/src/main.rs Cargo.lock
git commit -m "feat(tauri): export_text encoder + export_collection atomic-write command

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 5: frontend client wrappers + types + clipboard helper

Add the four client wrappers, a thin `copyText` clipboard helper, and mocked-invoke/mocked-plugin tests.

**Files:**
- Modify: `src/api/client.ts`, `src/api/client.test.ts`
- Create: `src/lib/clipboard.ts`, `src/lib/clipboard.test.ts`

- [ ] **Step 1: Failing tests for the new client wrappers**

Append to `src/api/client.test.ts` (inside the existing `describe`):

```ts
  it("setCollectionPrompt invokes set_collection_prompt with name + template", async () => {
    invokeMock.mockResolvedValue({ items: [], collections: [], groups: [] });
    await setCollectionPrompt("Work", "My prompt");
    expect(invokeMock).toHaveBeenCalledWith("set_collection_prompt", {
      name: "Work",
      template: "My prompt",
    });
  });

  it("setCollectionPrompt passes null to clear the override", async () => {
    invokeMock.mockResolvedValue({ items: [], collections: [], groups: [] });
    await setCollectionPrompt("Work", null);
    expect(invokeMock).toHaveBeenCalledWith("set_collection_prompt", {
      name: "Work",
      template: null,
    });
  });

  it("exportCollection invokes export_collection with name/format/path", async () => {
    invokeMock.mockResolvedValue(undefined);
    await exportCollection("Work", "jsonl", "/tmp/Work.jsonl");
    expect(invokeMock).toHaveBeenCalledWith("export_collection", {
      name: "Work",
      format: "jsonl",
      path: "/tmp/Work.jsonl",
    });
  });
```

Extend the top import in `client.test.ts` to include the new wrappers:

```ts
import {
  getSnapshot,
  onStoreChanged,
  createItem,
  setStatus,
  getSettings,
  setSettings,
  setCollectionPrompt,
  exportCollection,
} from "./client";
```

Run:

```bash
npx vitest run src/api/client.test.ts
```
Expected: **fail** (`setCollectionPrompt`/`exportCollection` not exported).

- [ ] **Step 2: Add the four wrappers to `client.ts`**

Append a new section to `src/api/client.ts` (after the `// --- Settings ---` block):

```ts
// --- Prompts / export ---
export function setCollectionPrompt(name: string, template: string | null): Promise<Snapshot> {
  return invoke<Snapshot>("set_collection_prompt", { name, template });
}

export function collectionPromptText(name: string): Promise<string> {
  return invoke<string>("collection_prompt_text", { name });
}

export function collectionCliCommand(name: string): Promise<string> {
  return invoke<string>("collection_cli_command", { name });
}

export function exportCollection(
  name: string,
  format: "json" | "jsonl",
  path: string,
): Promise<void> {
  return invoke<void>("export_collection", { name, format, path });
}
```

- [ ] **Step 3: Create `src/lib/clipboard.ts` + a failing test**

Create `src/lib/clipboard.test.ts`:

```ts
import { describe, expect, it, vi, beforeEach } from "vitest";

const writeTextMock = vi.fn();
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({
  writeText: (...a: unknown[]) => writeTextMock(...a),
}));

import { copyText } from "./clipboard";

describe("clipboard", () => {
  beforeEach(() => writeTextMock.mockReset());

  it("copyText calls the plugin writeText with the given string", async () => {
    writeTextMock.mockResolvedValue(undefined);
    await copyText("hello");
    expect(writeTextMock).toHaveBeenCalledWith("hello");
  });
});
```

Run:

```bash
npx vitest run src/lib/clipboard.test.ts
```
Expected: **fail** (`./clipboard` missing).

- [ ] **Step 4: Implement `src/lib/clipboard.ts`**

```ts
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

/** Write `text` to the system clipboard via the Tauri clipboard-manager plugin. */
export function copyText(text: string): Promise<void> {
  return writeText(text);
}
```

- [ ] **Step 5: Run tests (passing) + gate**

```bash
npx vitest run src/api/client.test.ts src/lib/clipboard.test.ts
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: green/clean. (If `writeText` is not the exported name in the installed `@tauri-apps/plugin-clipboard-manager`, adjust the import + the mock to the installed API; gate = clean build + green test.)

- [ ] **Step 6: Commit**

```bash
git add src/api/client.ts src/api/client.test.ts src/lib/clipboard.ts src/lib/clipboard.test.ts
git commit -m "feat(ui): prompt/export client wrappers + copyText clipboard helper

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 6: `PromptEditorDialog` + collection-menu prompt items

A new per-collection prompt editor (Radix `Dialog` + `TextArea` seeded from the collection's raw `promptTemplate`), hosted in `App`; plus the Sidebar collection-menu items **Edit Prompt…**, **Copy Prompt**, **Copy CLI Command**.

> **Radix 3.3.0 latitude:** uses `Dialog` (controlled `open`/`onOpenChange`, `Content`, `Title`, `Close`), `TextArea`, `Text`, `Flex`, `Button`. If 3.3.0's `TextArea`/`Dialog` prop surface differs, adjust to the installed API; gate is clean tsc+build.

**Files:**
- Create: `src/components/PromptEditorDialog.tsx`
- Modify: `src/App.tsx`, `src/components/Sidebar.tsx`

- [ ] **Step 1: Implement `PromptEditorDialog.tsx`**

```tsx
import { useEffect, useState } from "react";
import { Button, Dialog, Flex, Text, TextArea } from "@radix-ui/themes";
import type { Snapshot } from "../api/types";
import { setCollectionPrompt } from "../api/client";

export interface PromptEditorDialogProps {
  /** The collection whose override is being edited; null = closed. */
  collection: string | null;
  /** The collection's current raw override (may be empty/undefined). */
  initialTemplate?: string;
  onClose: () => void;
  onSnapshot: (snap: Snapshot) => void;
  onError: (msg: string) => void;
}

export function PromptEditorDialog({
  collection,
  initialTemplate,
  onClose,
  onSnapshot,
  onError,
}: PromptEditorDialogProps) {
  const [text, setText] = useState("");

  // Seed the editor from the collection's current override whenever it opens.
  useEffect(() => {
    if (collection !== null) setText(initialTemplate ?? "");
  }, [collection, initialTemplate]);

  const save = () => {
    if (collection === null) return;
    const trimmed = text.trim();
    setCollectionPrompt(collection, trimmed === "" ? null : text)
      .then((snap) => {
        onSnapshot(snap);
        onClose();
      })
      .catch((e) => onError(String(e)));
  };

  return (
    <Dialog.Root open={collection !== null} onOpenChange={(o) => { if (!o) onClose(); }}>
      <Dialog.Content maxWidth="560px">
        <Dialog.Title>Edit Prompt{collection ? ` — ${collection}` : ""}</Dialog.Title>
        <TextArea
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Prompt template…"
          rows={10}
        />
        <Text size="1" color="gray" mt="2" as="p">
          Leave empty to use the app default prompt.
        </Text>
        <Flex gap="2" mt="3" justify="end">
          <Dialog.Close>
            <Button variant="soft" color="gray">Cancel</Button>
          </Dialog.Close>
          <Button onClick={save}>Save</Button>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
}
```

- [ ] **Step 2: Host the editor in `App`**

Edit `src/App.tsx`. Add the import:

```tsx
import { PromptEditorDialog } from "./components/PromptEditorDialog";
```

Add the open-state (next to `settingsOpen`):

```tsx
  const [promptCollection, setPromptCollection] = useState<string | null>(null);
```

Compute the seeding template from the current snapshot (place inside `App`, before `return`):

```tsx
  const promptInitial =
    promptCollection === null
      ? undefined
      : snapshot.collections.find((c) => c.name === promptCollection)?.promptTemplate;
```

Mount the dialog inside the top-level `<Flex>` (next to `<SettingsDialog … />`):

```tsx
      <PromptEditorDialog
        collection={promptCollection}
        initialTemplate={promptInitial}
        onClose={() => setPromptCollection(null)}
        onSnapshot={apply}
        onError={onError}
      />
```

Pass an `onEditPrompt` callback to `<Sidebar …>` (add it alongside the existing Sidebar props):

```tsx
        onEditPrompt={(name) => setPromptCollection(name)}
```

- [ ] **Step 3: Add the three prompt items to the Sidebar collection menu**

Edit `src/components/Sidebar.tsx`. Extend the top client import to add the prompt/cli wrappers, and add the clipboard helper import:

```tsx
import {
  clearItems, collectionCliCommand, collectionPromptText, createCollection, createGroup,
  deleteCollection, deleteGroup, moveCollection, renameCollection, renameGroup,
  setCollectionArchived, setCollectionColor,
} from "../api/client";
import { copyText } from "../lib/clipboard";
```

Add `onEditPrompt` to `SidebarProps`:

```tsx
  onEditPrompt: (name: string) => void;
```

Destructure it in the function signature (add to the existing destructure list):

```tsx
export function Sidebar({
  snapshot, selected, showArchived, hideCompleted, usesAutoDraft, alwaysOnTop,
  onSelect, onToggleHideCompleted, onToggleShowArchived, onToggleAutoDraft, onToggleAlwaysOnTop,
  onOpenSettings, onEditPrompt, onSnapshot, onError, onRequestConfirm,
}: SidebarProps) {
```

In the collection `ContextMenu.Content` (the per-collection one starting at the `Rename` item), insert the three prompt items + a separator immediately after the `Rename` item and before the `Color` sub:

```tsx
                  <ContextMenu.Item onSelect={() => renameCol(c)}>Rename</ContextMenu.Item>
                  <ContextMenu.Item onSelect={() => onEditPrompt(c.name)}>Edit Prompt…</ContextMenu.Item>
                  <ContextMenu.Item
                    onSelect={() =>
                      collectionPromptText(c.name).then(copyText).catch((e) => onError(String(e)))
                    }
                  >
                    Copy Prompt
                  </ContextMenu.Item>
                  <ContextMenu.Item
                    onSelect={() =>
                      collectionCliCommand(c.name).then(copyText).catch((e) => onError(String(e)))
                    }
                  >
                    Copy CLI Command
                  </ContextMenu.Item>
                  <ContextMenu.Separator />
```

(Parity: Swift `CollectionMenus` exposes Copy Prompt, Edit Prompt…, Copy CLI Command on the collection menu; `collectionPromptText`/`collectionCliCommand` resolve server-side, the frontend only copies the returned string. `onError` matches the 5A error-dialog seam.)

- [ ] **Step 4: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green. (Report any `TextArea`/`Dialog` prop adjustment for 3.3.0.)

- [ ] **Step 5: Commit**

```bash
git add src/components/PromptEditorDialog.tsx src/App.tsx src/components/Sidebar.tsx
git commit -m "feat(ui): prompt editor dialog + collection-menu prompt/copy items

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 7: Export submenu + save

Add the **Export Collection** submenu (**As JSON** / **As JSONL**) to the Sidebar collection menu; each entry opens the dialog plugin's save picker and, on a non-null path, calls `exportCollection`.

> **Radix/plugin latitude:** uses `ContextMenu.Sub`/`SubTrigger`/`SubContent`/`Item` (already used for Color/Clear) + the dialog plugin `save`. If `@tauri-apps/plugin-dialog`'s `save` option/return shape differs in the installed version, adjust to the installed API; gate = clean tsc+build + the picker working at manual launch.

**Files:**
- Modify: `src/components/Sidebar.tsx`

- [ ] **Step 1: Import the export wrapper + the dialog plugin `save`, and add an export helper**

Edit `src/components/Sidebar.tsx`. Extend the client import to add `exportCollection`, and add the dialog-plugin import:

```tsx
import {
  clearItems, collectionCliCommand, collectionPromptText, createCollection, createGroup,
  deleteCollection, deleteGroup, exportCollection, moveCollection, renameCollection,
  renameGroup, setCollectionArchived, setCollectionColor,
} from "../api/client";
import { copyText } from "../lib/clipboard";
import { save } from "@tauri-apps/plugin-dialog";
```

Add an export helper inside the `Sidebar` function body (next to `renameCol`/`renameGrp`):

```tsx
  const exportAs = (name: string, format: "json" | "jsonl") => {
    const ext = format; // "json" | "jsonl"
    save({
      defaultPath: `${name}.${ext}`,
      filters: [{ name: format.toUpperCase(), extensions: [ext] }],
    })
      .then((path) => {
        if (path) return exportCollection(name, format, path);
      })
      .catch((e) => onError(String(e)));
  };
```

(Parity: Swift `CollectionExport.choose` sets `nameFieldStringValue = "<safeFilename>.json"` and offers JSON/JSONL; the Tauri submenu replaces the macOS save-panel format accessory, per design decision 5. A null `path` (user cancelled) is a no-op.)

- [ ] **Step 2: Add the Export Collection submenu**

In the collection `ContextMenu.Content`, after the existing `Clear` sub (and before the `Separator` that precedes `Delete`), add:

```tsx
                  <ContextMenu.Sub>
                    <ContextMenu.SubTrigger>Export Collection</ContextMenu.SubTrigger>
                    <ContextMenu.SubContent>
                      <ContextMenu.Item onSelect={() => exportAs(c.name, "json")}>
                        As JSON
                      </ContextMenu.Item>
                      <ContextMenu.Item onSelect={() => exportAs(c.name, "jsonl")}>
                        As JSONL
                      </ContextMenu.Item>
                    </ContextMenu.SubContent>
                  </ContextMenu.Sub>
```

- [ ] **Step 3: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green.

- [ ] **Step 4: Commit**

```bash
git add src/components/Sidebar.tsx
git commit -m "feat(ui): export collection submenu (JSON/JSONL) with save dialog

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 8: item-menu Copy ID

Add a **Copy ID** item to the task row's `ContextMenu` that copies the item's id to the clipboard.

**Files:**
- Modify: `src/components/TaskRow.tsx`

- [ ] **Step 1: Import the clipboard helper + add the menu item**

Edit `src/components/TaskRow.tsx`. Add the import:

```tsx
import { copyText } from "../lib/clipboard";
```

In the item `ContextMenu.Content`, add a **Copy ID** item after the `Move to Collection` sub and before the `Separator` that precedes `Delete`:

```tsx
        <ContextMenu.Item
          onSelect={() => copyText(item.id).catch((e) => onError(String(e)))}
        >
          Copy ID
        </ContextMenu.Item>
```

(Parity: Swift `TaskRow.copyIDToPasteboard` copies `item.id`. `onError` is already a `TaskRowProps` prop from 5A.)

- [ ] **Step 2: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green.

- [ ] **Step 3: Commit**

```bash
git add src/components/TaskRow.tsx
git commit -m "feat(ui): item-menu Copy ID

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 9: Settings Prompt tab

Add a **Prompt** tab to `SettingsDialog` with a `TextArea` seeded from `settings.defaultPromptTemplate`; Save persists it via `updateSettings`. Thread `settings`/`updateSettings` into the dialog (they are not currently passed).

> **Radix 3.3.0 latitude:** the dialog already uses `Tabs.Root`/`List`/`Trigger`/`Content`; add a second `Trigger`/`Content`. If 3.3.0's `Tabs`/`TextArea` shape differs, adjust to the installed API; gate is clean tsc+build.

**Files:**
- Modify: `src/components/SettingsDialog.tsx`, `src/App.tsx`

- [ ] **Step 1: Thread `settings`/`updateSettings` into `SettingsDialog` and add the Prompt tab**

Edit `src/components/SettingsDialog.tsx`. Extend imports + props, add Prompt-tab local state, and render the tab. The new top imports + props:

```tsx
import { useEffect, useState } from "react";
import { Button, Dialog, Flex, Tabs, Text, TextArea } from "@radix-ui/themes";
import { getVersion } from "@tauri-apps/api/app";
import { storePath } from "../api/client";
import type { Settings } from "../api/types";

export interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  settings: Settings;
  updateSettings: (patch: Partial<Settings>) => void;
}

export function SettingsDialog({ open, onOpenChange, settings, updateSettings }: SettingsDialogProps) {
  const [version, setVersion] = useState<string>("");
  const [path, setPath] = useState<string>("");
  const [promptDraft, setPromptDraft] = useState<string>("");
```

Add a seeding effect for the prompt draft (when the dialog opens), after the existing version/path effect:

```tsx
  // Seed the default-template editor from settings whenever the dialog opens.
  useEffect(() => {
    if (open) setPromptDraft(settings.defaultPromptTemplate);
  }, [open, settings.defaultPromptTemplate]);
```

Add a Prompt `Tabs.Trigger` next to the System Information trigger:

```tsx
          <Tabs.List>
            <Tabs.Trigger value="system">System Information</Tabs.Trigger>
            <Tabs.Trigger value="prompt">Prompt</Tabs.Trigger>
          </Tabs.List>
```

Add the Prompt `Tabs.Content` after the existing `system` content:

```tsx
          <Tabs.Content value="prompt">
            <Flex direction="column" gap="2" mt="3">
              <Text size="2" color="gray">Default prompt template</Text>
              <TextArea
                value={promptDraft}
                onChange={(e) => setPromptDraft(e.target.value)}
                placeholder="Default prompt template…"
                rows={10}
              />
              <Text size="1" color="gray">
                Leave empty to use the built-in default. Collections without their own
                prompt use this template.
              </Text>
              <Flex justify="end">
                <Button onClick={() => updateSettings({ defaultPromptTemplate: promptDraft })}>
                  Save
                </Button>
              </Flex>
            </Flex>
          </Tabs.Content>
```

(Parity: Swift `TaskPromptSettings.setDefaultPromptTemplate` stores the raw text; empty → the built-in default applies via the 3-level precedence. `updateSettings` is the 5A merge helper — it persists the whole `Settings` object.)

- [ ] **Step 2: Pass `settings`/`updateSettings` from `App`**

Edit `src/App.tsx`. Update the `<SettingsDialog …>` mount to pass the two new props:

```tsx
      <SettingsDialog
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        settings={settings}
        updateSettings={updateSettings}
      />
```

- [ ] **Step 3: Gate**

```bash
npx tsc --noEmit && npm run build && npx vitest run
```
Expected: clean/green. (Report any `Tabs`/`TextArea` 3.3.0 adjustment.)

- [ ] **Step 4: Commit**

```bash
git add src/components/SettingsDialog.tsx src/App.tsx
git commit -m "feat(ui): Settings Prompt tab editing the default template

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Done criteria

- `cargo test -p pond-tauri` green (incl. `prompt::tests` (9), `mutations` set/clear + 3 `export_text` cases, `commands` precedence); `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- `npx vitest run` green (incl. the new `client.test.ts` cases + `clipboard.test.ts`); `npx tsc --noEmit` clean; `npm run build` succeeds.
- The capability schema (`src-tauri/gen/schemas/desktop-schema.json`) lists `clipboard-manager:allow-write-text` and `dialog:allow-save` (or the confirmed equivalents) and `default.json` grants them.
- Manual `cargo tauri dev` launch (human check): the collection menu shows Edit Prompt… / Copy Prompt / Copy CLI Command / Export Collection ▸ As JSON|As JSONL; editing a collection prompt persists and seeds on reopen; an empty editor clears the override (falls back to default); Copy Prompt / Copy CLI Command / item Copy ID put the expected text on the clipboard; Export writes a `.json`/`.jsonl` file at the chosen path with the right shape; the Settings Prompt tab edits the app-default template and persists across relaunch; a forced command error surfaces in the error dialog.
