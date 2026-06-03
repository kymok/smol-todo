use crate::commands::build_snapshot;
use crate::dto::SnapshotDto;
use pond_core::{Result, TaskStatus, TaskStore, DEFAULT_COLLECTION};

/// Create a new empty Draft (title typed in the editor). `collection` is the
/// target collection api-name; `None`/empty falls back to the default collection.
pub fn create_item(store: &TaskStore, collection: Option<&str>) -> Result<SnapshotDto> {
    let target = collection
        .filter(|c| !c.is_empty())
        .unwrap_or(DEFAULT_COLLECTION);
    store.add("", target, None, true, TaskStatus::Draft)?;
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
}
