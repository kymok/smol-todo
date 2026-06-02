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

#[cfg(unix)]
pub mod cli_install;

#[cfg(test)]
mod api_smoke {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn end_to_end_via_public_api() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let item = store
            .add("Write spec", "Work/Docs", None, false, TaskStatus::Ready)
            .unwrap();
        store.add_note(&item.id, "outline first").unwrap();
        let summaries = store.collection_summaries().unwrap();
        assert!(summaries
            .iter()
            .any(|c: &CollectionSummary| c.name == "Work/Docs"));
        let groups: Vec<CollectionGroupSummary> = store.collection_group_summaries().unwrap();
        assert!(groups.iter().any(|g| g.name == "Work"));
    }
}
