use crate::dto::{CollectionGroupSummaryDto, CollectionSummaryDto, SnapshotDto};
use crate::mutations;
use crate::prompt;
use crate::settings::{self, Settings};
use pond_core::export::ExportFormat;
use pond_core::{CollectionColor, Result, TaskItem, TaskStatus, TaskStore};
use std::collections::HashMap;
use std::sync::Mutex;
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
pub fn delete_item(
    store: State<TaskStore>,
    id: String,
) -> std::result::Result<SnapshotDto, String> {
    mutations::delete_item(&store, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_items(
    store: State<TaskStore>,
    ids: Vec<String>,
) -> std::result::Result<SnapshotDto, String> {
    mutations::delete_items(&store, &ids).map_err(|e| e.to_string())
}

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
    mutations::split_item(
        &store,
        &id,
        &first_title,
        &second_title,
        second_id.as_deref(),
    )
    .map_err(|e| e.to_string())
}

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

#[tauri::command]
pub fn create_group(
    store: State<TaskStore>,
    name: String,
) -> std::result::Result<SnapshotDto, String> {
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
pub fn delete_group(
    store: State<TaskStore>,
    name: String,
) -> std::result::Result<SnapshotDto, String> {
    mutations::delete_group(&store, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(state: State<Mutex<Settings>>) -> std::result::Result<Settings, String> {
    let guard = state.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn set_settings(
    state: State<Mutex<Settings>>,
    settings: Settings,
) -> std::result::Result<Settings, String> {
    settings::save(&settings::settings_path(), &settings).map_err(|e| e.to_string())?;
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    *guard = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub fn store_path(store: State<TaskStore>) -> String {
    store.file_path().display().to_string()
}

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

#[cfg(unix)]
fn build_installer() -> pond_core::cli_install::Installer {
    use std::path::PathBuf;
    let home = directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let link = home.join(".local/bin/taskpond");
    let target =
        crate::install::resolve_taskpond_target().unwrap_or_else(|| PathBuf::from("taskpond"));
    let record = pond_core::paths::data_directory().join("cli-install.json");
    pond_core::cli_install::Installer::new(link, target, record)
}

#[cfg(unix)]
#[tauri::command]
pub fn cli_install_status() -> std::result::Result<crate::install::InstallStatusDto, String> {
    let installer = build_installer();
    Ok(crate::install::dto_from(
        &installer.status(),
        installer.path_hint(),
    ))
}

#[cfg(unix)]
#[tauri::command]
pub fn cli_install() -> std::result::Result<crate::install::InstallStatusDto, String> {
    let installer = build_installer();
    installer.install().map_err(|e| e.to_string())?;
    Ok(crate::install::dto_from(
        &installer.status(),
        installer.path_hint(),
    ))
}

#[cfg(unix)]
#[tauri::command]
pub fn cli_uninstall() -> std::result::Result<crate::install::InstallStatusDto, String> {
    let installer = build_installer();
    installer.uninstall().map_err(|e| e.to_string())?;
    Ok(crate::install::dto_from(
        &installer.status(),
        installer.path_hint(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pond_core::prompt::APPLICATION_DEFAULT_TEMPLATE;
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
}
