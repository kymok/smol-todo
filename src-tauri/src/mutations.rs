use crate::commands::build_snapshot;
use crate::dto::SnapshotDto;
use chrono::Utc;
use pond_core::export::{ExportFormat, ExportPayload};
use pond_core::{
    CollectionColor, Result, TaskItem, TaskStatus, TaskStore, DEFAULT_COLLECTION, DEFAULT_GROUP,
};
use std::collections::HashMap;

/// Create a new empty Draft (title typed in the editor). `collection` is the
/// target collection api-name; `None`/empty falls back to the default collection.
pub fn create_item(store: &TaskStore, collection: Option<&str>) -> Result<SnapshotDto> {
    let target = collection
        .filter(|c| !c.is_empty())
        .unwrap_or(DEFAULT_COLLECTION);
    store.add("", target, None, true, TaskStatus::Draft)?;
    build_snapshot(store)
}

pub fn update_item(
    store: &TaskStore,
    id: &str,
    title: Option<&str>,
    collection: Option<&str>,
    status: Option<TaskStatus>,
    if_current: Option<TaskItem>,
) -> Result<SnapshotDto> {
    match if_current {
        Some(expected) => {
            store.update_if_current(id, title, collection, status, &expected)?;
        }
        None => {
            store.update(id, title, collection, status)?;
        }
    }
    build_snapshot(store)
}

pub fn set_status(
    store: &TaskStore,
    status: TaskStatus,
    id: &str,
    if_current: Option<TaskItem>,
) -> Result<SnapshotDto> {
    match if_current {
        Some(expected) => {
            store.set_status_if_current(status, id, &expected)?;
        }
        None => {
            store.set_status(status, &[id.to_string()], None)?;
        }
    }
    build_snapshot(store)
}

pub fn move_item(store: &TaskStore, id: &str, collection: &str) -> Result<SnapshotDto> {
    store.move_item(id, collection)?;
    build_snapshot(store)
}

pub fn delete_item(store: &TaskStore, id: &str) -> Result<SnapshotDto> {
    store.delete(id)?;
    build_snapshot(store)
}

pub fn delete_items(store: &TaskStore, ids: &[String]) -> Result<SnapshotDto> {
    store.delete_many(ids, None)?;
    build_snapshot(store)
}

pub fn add_note(
    store: &TaskStore,
    id: &str,
    body: &str,
    if_current: Option<TaskItem>,
) -> Result<SnapshotDto> {
    match if_current {
        Some(expected) => {
            store.add_note_if_current(id, body, &expected)?;
        }
        None => {
            store.add_note(id, body)?;
        }
    }
    build_snapshot(store)
}

pub fn update_note(store: &TaskStore, id: &str, body: &str) -> Result<SnapshotDto> {
    store.update_note(id, body)?;
    build_snapshot(store)
}

pub fn delete_note(
    store: &TaskStore,
    id: &str,
    if_current: Option<TaskItem>,
) -> Result<SnapshotDto> {
    match if_current {
        Some(expected) => {
            store.delete_note_if_current(id, &expected)?;
        }
        None => {
            store.delete_note(id)?;
        }
    }
    build_snapshot(store)
}

pub fn merge_item(
    store: &TaskStore,
    id: &str,
    into_previous: &str,
    title: &str,
) -> Result<SnapshotDto> {
    store.merge_item(id, into_previous, title)?;
    build_snapshot(store)
}

pub fn split_item(
    store: &TaskStore,
    id: &str,
    first_title: &str,
    second_title: &str,
    second_id: Option<&str>,
) -> Result<SnapshotDto> {
    store.split_item(id, first_title, second_title, second_id)?;
    build_snapshot(store)
}

pub fn create_collection(
    store: &TaskStore,
    name: &str,
    group: Option<&str>,
) -> Result<SnapshotDto> {
    let group = group.filter(|g| !g.is_empty()).unwrap_or(DEFAULT_GROUP);
    store.create_collection(name, group)?;
    build_snapshot(store)
}

pub fn rename_collection(store: &TaskStore, old: &str, new: &str) -> Result<SnapshotDto> {
    store.rename_collection(old, new)?;
    build_snapshot(store)
}

pub fn set_collection_color(
    store: &TaskStore,
    name: &str,
    color: CollectionColor,
) -> Result<SnapshotDto> {
    store.set_collection_color(name, color)?;
    build_snapshot(store)
}

pub fn set_collection_archived(
    store: &TaskStore,
    name: &str,
    is_archived: bool,
) -> Result<SnapshotDto> {
    store.set_collection_archived(name, is_archived)?;
    build_snapshot(store)
}

pub fn move_collection(store: &TaskStore, name: &str, group: &str) -> Result<SnapshotDto> {
    store.move_collection(name, group)?;
    build_snapshot(store)
}

pub fn clear_items(store: &TaskStore, name: &str, completed_only: bool) -> Result<SnapshotDto> {
    store.clear_items(name, completed_only)?;
    build_snapshot(store)
}

pub fn delete_collection(store: &TaskStore, name: &str) -> Result<SnapshotDto> {
    store.delete_collection(name)?;
    build_snapshot(store)
}

pub fn create_group(store: &TaskStore, name: &str) -> Result<SnapshotDto> {
    store.create_group(name)?;
    build_snapshot(store)
}

pub fn rename_group(store: &TaskStore, old: &str, new: &str) -> Result<SnapshotDto> {
    store.rename_group(old, new)?;
    build_snapshot(store)
}

pub fn delete_group(store: &TaskStore, name: &str) -> Result<SnapshotDto> {
    store.delete_group(name)?;
    build_snapshot(store)
}

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

/// Remap statuses within a single collection: every item whose current status is a key
/// in `replacements` is set to the mapped value (no-op pairs are ignored by pond-core).
/// `ids` empty + `Some(collection)` scopes it to the whole collection. Returns the snapshot.
pub fn set_statuses(
    store: &TaskStore,
    replacements: &HashMap<TaskStatus, TaskStatus>,
    collection: &str,
) -> Result<SnapshotDto> {
    store.set_statuses(replacements, &[], Some(collection))?;
    build_snapshot(store)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use pond_core::export::ExportFormat;
    use pond_core::{CollectionColor, TaskStatus, DEFAULT_GROUP};
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn store() -> (tempfile::TempDir, TaskStore) {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        (dir, store)
    }

    #[test]
    fn create_item_adds_a_draft() {
        let (_dir, store) = store();
        let snap = create_item(&store, None).unwrap();
        assert_eq!(snap.items.len(), 1);
        assert_eq!(snap.items[0].status, TaskStatus::Draft);
        assert_eq!(snap.items[0].title, "");
        assert_eq!(snap.items[0].collection, "Inbox");
    }

    #[test]
    fn create_item_honors_explicit_collection() {
        let (_dir, store) = store();
        let snap = create_item(&store, Some("Work/Docs")).unwrap();
        assert_eq!(snap.items.len(), 1);
        assert_eq!(snap.items[0].collection, "Work/Docs");
    }

    fn seed(store: &TaskStore, title: &str) -> pond_core::TaskItem {
        store
            .add(title, "Inbox", None, false, TaskStatus::Ready)
            .unwrap()
    }

    #[test]
    fn update_item_changes_title_and_status() {
        let (_dir, store) = store();
        let item = seed(&store, "old");
        let snap = update_item(
            &store,
            &item.id,
            Some("new"),
            None,
            Some(TaskStatus::OnHold),
            None,
        )
        .unwrap();
        let got = &snap.items[0];
        assert_eq!(got.title, "new");
        assert_eq!(got.status, TaskStatus::OnHold);
    }

    #[test]
    fn update_item_if_current_skips_stale() {
        let (_dir, store) = store();
        let item = seed(&store, "old");
        store
            .update(&item.id, Some("changed-out-of-band"), None, None)
            .unwrap();
        // `item` is now stale; the guarded update must be a no-op.
        let snap = update_item(
            &store,
            &item.id,
            Some("ignored"),
            None,
            None,
            Some(item.clone()),
        )
        .unwrap();
        assert_eq!(snap.items[0].title, "changed-out-of-band");
    }

    #[test]
    fn set_status_single_and_if_current() {
        let (_dir, store) = store();
        let item = seed(&store, "t");
        let snap = set_status(&store, TaskStatus::Completed, &item.id, None).unwrap();
        assert_eq!(snap.items[0].status, TaskStatus::Completed);

        let current = snap.items[0].clone();
        let snap = set_status(
            &store,
            TaskStatus::Ready,
            &current.id,
            Some(current.clone()),
        )
        .unwrap();
        assert_eq!(snap.items[0].status, TaskStatus::Ready);
    }

    #[test]
    fn set_status_if_current_skips_stale() {
        let (_dir, store) = store();
        let item = seed(&store, "t");
        // Mutate out-of-band so the seeded handle becomes stale.
        store
            .set_status(TaskStatus::Completed, std::slice::from_ref(&item.id), None)
            .unwrap();
        // Guarded call with the stale item must be a no-op.
        let snap = set_status(&store, TaskStatus::Ready, &item.id, Some(item.clone())).unwrap();
        assert_eq!(snap.items[0].status, TaskStatus::Completed);
    }

    #[test]
    fn move_item_changes_collection() {
        let (_dir, store) = store();
        let item = seed(&store, "t");
        let snap = move_item(&store, &item.id, "Work/Docs").unwrap();
        assert_eq!(snap.items[0].collection, "Work/Docs");
    }

    #[test]
    fn delete_item_and_delete_items() {
        let (_dir, store) = store();
        let a = seed(&store, "a");
        let b = seed(&store, "b");
        let snap = delete_item(&store, &a.id).unwrap();
        assert_eq!(snap.items.len(), 1);
        let snap = delete_items(&store, &[b.id]).unwrap();
        assert_eq!(snap.items.len(), 0);
    }

    #[test]
    fn add_update_delete_note() {
        let (_dir, store) = store();
        let item = seed(&store, "t");
        let snap = add_note(&store, &item.id, "first body", None).unwrap();
        assert_eq!(snap.items[0].note.as_ref().unwrap().body, "first body");

        let snap = update_note(&store, &item.id, "second body").unwrap();
        assert_eq!(snap.items[0].note.as_ref().unwrap().body, "second body");

        let snap = delete_note(&store, &item.id, None).unwrap();
        assert!(snap.items[0].note.is_none());
    }

    #[test]
    fn add_note_if_current_skips_stale() {
        let (_dir, store) = store();
        let item = seed(&store, "t");
        store
            .set_status(TaskStatus::OnHold, std::slice::from_ref(&item.id), None)
            .unwrap();
        // `item` stale → guarded add is a no-op, note stays absent.
        let snap = add_note(&store, &item.id, "ignored", Some(item.clone())).unwrap();
        assert!(snap.items[0].note.is_none());
    }

    #[test]
    fn merge_item_appends_into_previous_and_removes_source() {
        let (_dir, store) = store();
        let prev = store
            .add("Hello", "Inbox", None, false, TaskStatus::Ready)
            .unwrap();
        let src = store
            .add("World", "Inbox", None, false, TaskStatus::Ready)
            .unwrap();
        // merge_item appends `title` directly to prev's title via push_str.
        let snap = merge_item(&store, &src.id, &prev.id, " World").unwrap();
        assert_eq!(snap.items.len(), 1);
        assert_eq!(snap.items[0].id, prev.id);
        assert_eq!(snap.items[0].title, "Hello World");
    }

    #[test]
    fn split_item_creates_a_second_task() {
        let (_dir, store) = store();
        let item = store
            .add("alpha beta", "Inbox", None, false, TaskStatus::Ready)
            .unwrap();
        let snap = split_item(&store, &item.id, "alpha", "beta", None).unwrap();
        assert_eq!(snap.items.len(), 2);
        let titles: Vec<&str> = snap.items.iter().map(|i| i.title.as_str()).collect();
        assert!(titles.contains(&"alpha"));
        assert!(titles.contains(&"beta"));
    }

    #[test]
    fn delete_note_if_current_skips_stale() {
        let (_dir, store) = store();
        let item = seed(&store, "t");
        // Add a note so there is something to (attempt to) delete.
        store.add_note(&item.id, "body").unwrap();
        // Mutate out-of-band so the seeded handle becomes stale.
        store
            .set_status(TaskStatus::Completed, std::slice::from_ref(&item.id), None)
            .unwrap();
        // Guarded delete with the stale item must be a no-op; note stays present.
        let snap = delete_note(&store, &item.id, Some(item.clone())).unwrap();
        assert!(snap.items[0].note.is_some());
    }

    #[test]
    fn collection_lifecycle() {
        let (_dir, store) = store();
        // create
        let snap = create_collection(&store, "Errands", None).unwrap();
        assert!(snap.collections.iter().any(|c| c.name == "Errands"));
        // color
        let snap = set_collection_color(&store, "Errands", CollectionColor::Blue).unwrap();
        assert_eq!(
            snap.collections
                .iter()
                .find(|c| c.name == "Errands")
                .unwrap()
                .color,
            CollectionColor::Blue
        );
        // archive
        let snap = set_collection_archived(&store, "Errands", true).unwrap();
        assert!(
            snap.collections
                .iter()
                .find(|c| c.name == "Errands")
                .unwrap()
                .is_archived
        );
        // rename
        let snap = rename_collection(&store, "Errands", "Tasks").unwrap();
        assert!(snap.collections.iter().any(|c| c.name == "Tasks"));
        assert!(!snap.collections.iter().any(|c| c.name == "Errands"));
        // move to a group (api-name becomes "Work/Tasks")
        let snap = move_collection(&store, "Tasks", "Work").unwrap();
        assert!(snap.collections.iter().any(|c| c.name == "Work/Tasks"));
        // delete
        let snap = delete_collection(&store, "Work/Tasks").unwrap();
        assert!(!snap.collections.iter().any(|c| c.name == "Work/Tasks"));
    }

    #[test]
    fn clear_items_removes_completed_only() {
        let (_dir, store) = store();
        store.create_collection("Box", DEFAULT_GROUP).unwrap();
        store
            .add("keep", "Box", None, false, TaskStatus::Ready)
            .unwrap();
        store
            .add("drop", "Box", None, false, TaskStatus::Completed)
            .unwrap();
        let snap = clear_items(&store, "Box", true).unwrap();
        assert_eq!(
            snap.items.iter().filter(|i| i.collection == "Box").count(),
            1
        );
        assert_eq!(snap.items[0].title, "keep");
    }

    #[test]
    fn group_lifecycle() {
        let (_dir, store) = store();
        let snap = create_group(&store, "Personal").unwrap();
        assert!(snap.groups.iter().any(|g| g.name == "Personal"));
        let snap = rename_group(&store, "Personal", "Home").unwrap();
        assert!(snap.groups.iter().any(|g| g.name == "Home"));
        assert!(!snap.groups.iter().any(|g| g.name == "Personal"));
        let snap = delete_group(&store, "Home").unwrap();
        assert!(!snap.groups.iter().any(|g| g.name == "Home"));
    }

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
        store.create_collection("Empty", DEFAULT_GROUP).unwrap();

        // JSONL of an empty collection is the empty string (pond-core contract).
        let jsonl = export_text(&store, "Empty", ExportFormat::Jsonl).unwrap();
        assert_eq!(jsonl, "");

        // JSON still emits the wrapper with an empty items array.
        let json = export_text(&store, "Empty", ExportFormat::Json).unwrap();
        assert!(json.contains("\"items\""));
        assert!(json.contains("\"Empty\""));
    }

    #[test]
    fn set_statuses_remaps_within_collection() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add("A", "Work", None, false, TaskStatus::Ready)
            .unwrap();
        store
            .add("B", "Work", None, false, TaskStatus::Ready)
            .unwrap();
        store
            .add("C", "Work", None, false, TaskStatus::InProgress)
            .unwrap();
        // A different collection that must be untouched.
        store
            .add("X", "Home", None, false, TaskStatus::Ready)
            .unwrap();

        let mut replacements = HashMap::new();
        replacements.insert(TaskStatus::Ready, TaskStatus::Completed);
        let snap = set_statuses(&store, &replacements, "Work").unwrap();

        // Work: the two Ready items are now Completed; the InProgress is unchanged.
        let work: Vec<&TaskItem> = snap
            .items
            .iter()
            .filter(|i| i.collection == "Work")
            .collect();
        assert_eq!(
            work.iter()
                .filter(|i| i.status == TaskStatus::Completed)
                .count(),
            2
        );
        assert_eq!(
            work.iter()
                .filter(|i| i.status == TaskStatus::InProgress)
                .count(),
            1
        );
        assert_eq!(
            work.iter()
                .filter(|i| i.status == TaskStatus::Ready)
                .count(),
            0
        );

        // Home is untouched (still Ready).
        let home: Vec<&TaskItem> = snap
            .items
            .iter()
            .filter(|i| i.collection == "Home")
            .collect();
        assert_eq!(home.len(), 1);
        assert_eq!(home[0].status, TaskStatus::Ready);
    }

    #[test]
    fn replacements_map_deserializes_from_json_object() {
        // The wire shape is a JS object of status-string -> status-string.
        let map: HashMap<TaskStatus, TaskStatus> =
            serde_json::from_str(r#"{"ready":"completed","in-progress":"on-hold"}"#).unwrap();
        assert_eq!(map.get(&TaskStatus::Ready), Some(&TaskStatus::Completed));
        assert_eq!(map.get(&TaskStatus::InProgress), Some(&TaskStatus::OnHold));
    }
}
