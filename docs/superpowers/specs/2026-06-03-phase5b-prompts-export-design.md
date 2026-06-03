# Phase 5B: Prompts + Export + Clipboard — Design

- **Date:** 2026-06-03
- **Status:** Approved design
- **Phase:** 5B of 5 (Phase 5 split into 5A/5B/5C; see `2026-06-02-tauri-radix-migration-design.md` §9.5)
- **Builds on:** Phases 1–4 + Phase 5A complete (settings store + auto-draft + Settings dialog shell + error dialogs).

## 1. Overview

The "get data out" cluster: per-collection prompts (editor + Copy Prompt / Copy CLI Command),
per-collection export (JSON/JSONL to a file), and clipboard (Copy ID / Copy Prompt / Copy CLI
Command). Adds the Tauri **clipboard** + **dialog** plugins and the Settings **Prompt** tab.
Most logic already lives in `pond-core` (`prompt::evaluate` + `APPLICATION_DEFAULT_TEMPLATE`,
`ExportPayload.encode`, `set_collection_prompt`); 5B is the IPC + UI wiring.

### Goals
- Per-collection prompt override editor; the Settings Prompt tab edits the app default template.
- Copy Prompt (evaluated template) and Copy CLI Command (the `taskpond` command) from the
  collection menu; Copy ID from the item menu — all via the clipboard plugin.
- Export a collection to JSON or JSONL via a save dialog.

### Non-goals (deferred to 5C)
- CLI-install UI + `taskpond` sidecar packaging, file-drop-to-create, bulk-status dialog, the
  Settings **Command** tab.
- Auto-draft, always-on-top, error dialogs, the settings store (all done in 5A).

## 2. Confirmed decisions

1. **Two plugins:** `tauri-plugin-clipboard-manager` (Copy actions) and `tauri-plugin-dialog`
   (export save picker), with matching capability permissions. No `fs` plugin — export file IO
   stays in Rust (atomic write), the dialog only picks the path.
2. **Prompt text resolved server-side.** `collection_prompt_text` computes the 3-level
   precedence + `evaluate` in Rust (it needs both the store and the settings); the frontend
   only writes the returned string to the clipboard. `collection_cli_command` likewise (it
   needs shell-escaping).
3. **3-level prompt precedence** (port of Swift `effectivePromptTemplate` /
   `effectiveDefaultPromptTemplate`): the collection's `promptTemplate` if non-empty (trimmed),
   else `settings.defaultPromptTemplate` if non-empty (trimmed), else
   `APPLICATION_DEFAULT_TEMPLATE`. Evaluation substitutes **two** variables:
   `cliCommand` and `collectionName` (port of `taskExamplePrompt`).
4. **CLI command:** `taskpond item get --collection <shell-escaped name>` (port Swift's
   `shellEscaped`).
5. **Export format choice via a Radix submenu** ("Export Collection ▸ As JSON / As JSONL"),
   matching the existing Clear submenu — instead of replicating macOS's save-panel format
   accessory. The chosen format + `dialog.save({ defaultPath })` → a Rust command writes the
   encoded payload atomically.
6. Radix defaults only; no visual/DOM tests (logic in Rust/client wrappers is unit-tested;
   editors/dialogs/clipboard verified at manual launch).

## 3. Architecture

```
src-tauri/
├─ Cargo.toml      + tauri-plugin-clipboard-manager, tauri-plugin-dialog
├─ capabilities/default.json  + clipboard write-text + dialog save permissions
└─ src/
   ├─ main.rs      .plugin(tauri_plugin_clipboard_manager::init()).plugin(tauri_plugin_dialog::init()); register the new commands
   ├─ prompt.rs    (NEW) pure: effective_default_template + effective_collection_template (3-level) + cli_command + shell_escape
   ├─ commands.rs  + set_collection_prompt / collection_prompt_text / collection_cli_command / export_collection
   ├─ mutations.rs + set_collection_prompt (snapshot) + export_text(store,name,format)->String
   └─ export-write (in commands or mutations): write export_text to a path atomically

src/
├─ api/client.ts   + setCollectionPrompt / collectionPromptText / collectionCliCommand / exportCollection wrappers
├─ lib/clipboard.ts (NEW, thin) writeText via @tauri-apps/plugin-clipboard-manager
├─ components/
│  ├─ PromptEditorDialog.tsx (NEW) Radix Dialog + TextArea for a per-collection prompt
│  ├─ SettingsDialog.tsx     + Prompt tab (default template editor)
│  ├─ Sidebar.tsx            collection menu: Edit Prompt…, Copy Prompt, Copy CLI Command, Export ▸ As JSON/JSONL
│  └─ TaskRow.tsx            item menu: Copy ID
└─ App.tsx         host PromptEditorDialog state (which collection is being edited)
```

## 4. IPC commands (5B)

| Command | Behavior |
|---|---|
| `set_collection_prompt(name, template: Option<String>)` | pond-core `set_collection_prompt(name, template)` (None/empty clears the override) → rebuilt `SnapshotDto` (a mutation). |
| `collection_prompt_text(name) -> String` | resolve template (3-level precedence using the store's collection `promptTemplate` + `settings.defaultPromptTemplate` + `APPLICATION_DEFAULT_TEMPLATE`), then `prompt::evaluate(template, { "cliCommand": cli_command(name), "collectionName": name })`. Reads `State<TaskStore>` + `State<Mutex<Settings>>`. |
| `collection_cli_command(name) -> String` | `format!("taskpond item get --collection {}", shell_escape(name))`. |
| `export_collection(name, format: String, path: String) -> Result<(), String>` | parse `format` ("json"/"jsonl") → `ExportFormat`; `export_text(store, name, fmt)` builds `ExportPayload { collection: name, exported_at: Utc::now(), items: <collection's items> }` and `encode`; write the string atomically to `path` (temp+rename). |

- **`prompt.rs`** holds the pure pieces (no `Settings`/`TaskStore` deps — plain string inputs, unit-tested without Tauri): `effective_default_template(stored_default: &str) -> &str` (stored if non-empty after trim, else `APPLICATION_DEFAULT_TEMPLATE`); `effective_collection_template(collection_template: Option<&str>, stored_default: &str) -> String` (collection template if non-empty after trim, else `effective_default_template(stored_default)`); `cli_command(name: &str) -> String`; `shell_escape(name: &str) -> String` (port Swift `shellEscaped`). The `collection_prompt_text` command resolves the strings from the store + settings, then calls these helpers + `evaluate`.
- `set_collection_prompt` is added to `mutations.rs` (returns `SnapshotDto`); the others are read commands.
- `export_text(store, name, format)` lives in `mutations.rs` (testable, returns the encoded string); `export_collection` is the command that calls it + writes the file.

## 5. Prompt resolution detail (port of Swift)

- `effective_default_template(stored_default)` = `stored_default` if non-empty after trim,
  else `APPLICATION_DEFAULT_TEMPLATE`.
- `effective_collection_template(collection_template, stored_default)` = `collection_template`
  if non-empty after trim, else `effective_default_template(stored_default)`.
- The command computes:
  `let tmpl = effective_collection_template(collection.prompt_template.as_deref(), &settings.default_prompt_template);`
  then `evaluate(&tmpl, { "cliCommand": cli_command(name), "collectionName": name })`.
- The collection's current raw `promptTemplate` (for seeding the Edit Prompt… editor) comes
  from the existing `CollectionSummary.promptTemplate` already in the snapshot — no new command
  needed to read it.

## 6. Frontend

- **Clipboard wrapper** (`lib/clipboard.ts`): `copyText(text: string): Promise<void>` →
  `writeText` from `@tauri-apps/plugin-clipboard-manager`.
- **Collection menu** (Sidebar): add **Edit Prompt…** (opens `PromptEditorDialog` for that
  collection — App holds `{ promptCollection: string | null }`), **Copy Prompt**
  (`collectionPromptText(name)` → `copyText`), **Copy CLI Command** (`collectionCliCommand(name)`
  → `copyText`), and **Export Collection** submenu (**As JSON** / **As JSONL** → `dialog.save`
  → `exportCollection`). Errors route through the existing `onError`.
- **Item menu** (TaskRow): add **Copy ID** (`copyText(item.id)`).
- **`PromptEditorDialog`**: Radix `Dialog` + a `TextArea` seeded from the collection's
  `promptTemplate` (raw override, may be empty); a Save button → `setCollectionPrompt(name,
  text.trim() === "" ? null : text)` then `onSnapshot`; Cancel/Escape closes. A hint that an
  empty template falls back to the app default.
- **Settings Prompt tab** (`SettingsDialog`): a `TextArea` seeded from
  `settings.defaultPromptTemplate`; Save → `updateSettings({ defaultPromptTemplate: text })`
  (empty → built-in default). Add the "Prompt" `Tabs.Trigger`/`Content` alongside System
  Information.
- **Export save:** for the chosen format, `dialog.save({ defaultPath: "<name>.json", filters:
  [{ name: "JSON", extensions: ["json"] }] })` (or `.jsonl` / `extensions: ["jsonl"]` for JSONL);
  if the user cancels (`null`), do nothing; else `exportCollection(name, format, path)`.

## 7. Testing / verification

- **Rust (`pond-tauri`):** `prompt.rs` — `resolve` precedence (collection override / default
  setting / built-in, with trim), `shell_escape` (plain name, name with spaces/quotes),
  `cli_command` format; `mutations` — `export_text` JSON wrapper + JSONL one-per-line + empty,
  `set_collection_prompt` snapshot reflects/clears the override. (`collection_prompt_text`'s
  evaluation is covered by `prompt.rs` resolve + the existing `pond-core` `evaluate` tests.)
- **Frontend logic (Vitest):** client-wrapper tests (mocked `invoke`) for the new commands
  (right name + args), and a thin `copyText` test (mocked plugin).
- **Gates** stay green; **no visual/DOM tests** — the prompt editors, export save dialog, and
  clipboard are verified at manual launch.

## 8. References (source of truth)
- `Sources/PondApp/CollectionMenus.swift` — Copy Prompt / Edit Prompt… / Copy CLI Command /
  Export Collection…; `cliCommand`, `effectivePromptTemplate`, `examplePrompt`.
- `Sources/PondApp/TaskPromptSettings.swift` — `effectiveDefaultPromptTemplate`,
  `setDefaultPromptTemplate`, `PromptTemplateEditor`.
- `Sources/PondApp/TaskViewSupport.swift` — `taskExamplePrompt` (variables `cliCommand`,
  `collectionName`), `copyToPasteboard`.
- `Sources/PondApp/TaskRow.swift` — Copy ID (`copyIDToPasteboard`).
- `Sources/PondApp/CollectionExport.swift` — export formats (JSON/JSONL), save panel,
  `CollectionExportPayload { collection, exportedAt, items }`.
- `crates/pond-core/src/{prompt.rs,export.rs,store.rs}` — `evaluate`,
  `APPLICATION_DEFAULT_TEMPLATE`, `ExportPayload`, `set_collection_prompt`.
- Master spec `2026-06-02-tauri-radix-migration-design.md` §5 (IPC), §7 (prompts/export).
