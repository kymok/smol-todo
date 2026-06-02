use crate::dto::{CollectionGroupSummaryDto, CollectionSummaryDto, SnapshotDto};
use pond_core::{Result, TaskStore};

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
pub fn get_snapshot() -> std::result::Result<SnapshotDto, String> {
    build_snapshot(&TaskStore::open_default()).map_err(|e| e.to_string())
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
