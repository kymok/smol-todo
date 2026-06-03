use crate::dto::{CollectionGroupSummaryDto, CollectionSummaryDto, SnapshotDto};
use crate::mutations;
use pond_core::{CollectionColor, Result, TaskItem, TaskStatus, TaskStore};
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
