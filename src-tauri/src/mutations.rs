use crate::commands::build_snapshot;
use crate::dto::SnapshotDto;
use pond_core::{Result, TaskItem, TaskStatus, TaskStore, DEFAULT_COLLECTION};

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

#[cfg(test)]
mod tests {
    use super::*;
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
}
