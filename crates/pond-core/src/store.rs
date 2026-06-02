use crate::collections::{
    add_collection_if_missing, make_collection_group_summaries, make_collection_summaries,
    normalized_collection,
};
use crate::document::TaskFile;
use crate::error::{Result, StoreError};
use crate::ids::{is_valid_id, make_id, make_version};
use crate::json::to_pretty_sorted;
use crate::model::{CollectionGroupSummary, CollectionSummary, TaskItem, TaskStatus};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

pub struct TaskStore {
    file_path: PathBuf,
    lock_path: PathBuf,
}

impl TaskStore {
    pub fn new<P: Into<PathBuf>>(file_path: P) -> Self {
        let file_path = file_path.into();
        let lock_path = with_extension_suffix(&file_path, "lock");
        TaskStore {
            file_path,
            lock_path,
        }
    }

    pub fn open_default() -> Self {
        TaskStore::new(crate::paths::default_store_path())
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Run `body` under an exclusive advisory lock, writing the (possibly mutated)
    /// file back atomically when `write` is true.
    ///
    /// The parent directory is created up front (via `create_dir_all`) even for
    /// read-only calls, because the sibling `.lock` file must exist to take the
    /// lock. This mirrors the original app, whose `withFile` likewise created the
    /// directory before locking; a read therefore may create the `pond/` directory
    /// (but never `tasks.json` itself). The lock is held across the entire
    /// read -> body -> write sequence and released when `lock_file` is dropped.
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

        let mut file = self.read_file()?;
        let value = body(&mut file)?;
        if write {
            self.write_file(&file)?;
        }
        Ok(value)
        // lock_file dropped here → advisory lock released
    }

    fn read_file(&self) -> Result<TaskFile> {
        match fs::read(&self.file_path) {
            Ok(bytes) if bytes.is_empty() => Ok(TaskFile::default()),
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(TaskFile::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Persist the file atomically: write to a sibling `.tmp` file, then rename it
    /// over the real path (same directory, so the rename is atomic on POSIX). This
    /// does not `fsync` before the rename — an accepted durability trade-off for a
    /// task store, matching the original app's `Data.write(options: .atomic)`.
    fn write_file(&self, file: &TaskFile) -> Result<()> {
        let json = to_pretty_sorted(file)?;
        let tmp_path = with_extension_suffix(&self.file_path, "tmp");
        fs::write(&tmp_path, json.as_bytes())?;
        fs::rename(&tmp_path, &self.file_path)?;
        Ok(())
    }
}

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
                        || i.id.contains(&q)
                        || i.note
                            .as_ref()
                            .map_or(false, |n| n.body.to_lowercase().contains(&q))
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

impl TaskStore {
    pub fn add(
        &self,
        title: &str,
        collection: &str,
        requested_id: Option<&str>,
        allow_empty_title: bool,
        status: TaskStatus,
    ) -> Result<TaskItem> {
        let clean_title = if allow_empty_title {
            title.to_string()
        } else {
            title.trim().to_string()
        };
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
            let mut item = TaskItem::new(
                id,
                clean_title.clone(),
                clean_collection.clone(),
                status,
                now,
            );
            let existing_versions: HashSet<String> =
                file.items.iter().map(|i| i.version.clone()).collect();
            item.version = make_version(&existing_versions);
            file.items.push(item.clone());
            add_collection_if_missing(&item.collection, None, file)?;
            Ok(item)
        })
    }
}

impl TaskStore {
    fn mark_updated(file: &mut TaskFile, index: usize, now: DateTime<Utc>) {
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

fn with_extension_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::is_valid_id;
    use crate::model::TaskStatus;
    use chrono::{TimeZone, Utc};
    use tempfile::tempdir;

    #[test]
    fn reads_default_when_missing() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let count = store.with_file(false, |f| Ok(f.items.len())).unwrap();
        assert_eq!(count, 0);
        assert!(
            !dir.path().join("tasks.json").exists(),
            "read must not create the file"
        );
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
        let collections = store
            .with_file(false, |f| Ok(f.collections.clone()))
            .unwrap();
        assert_eq!(collections, vec!["Inbox".to_string()]);
    }

    #[test]
    fn read_only_does_not_persist() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .with_file(false, |f| {
                f.collections.push("Ghost".into());
                Ok(())
            })
            .unwrap();
        let collections = store
            .with_file(false, |f| Ok(f.collections.clone()))
            .unwrap();
        assert!(collections.is_empty());
    }

    fn seed(store: &TaskStore, items: &[(&str, &str, TaskStatus)]) {
        store
            .with_file(true, |f| {
                let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();
                for (id, collection, status) in items {
                    let mut it =
                        TaskItem::new((*id).into(), "t".into(), (*collection).into(), *status, now);
                    it.version = "v".repeat(12);
                    f.items.push(it);
                    f.collections = crate::collections::normalized_collection_list(
                        f.collections
                            .iter()
                            .cloned()
                            .chain([(*collection).to_string()])
                            .collect::<Vec<_>>(),
                    );
                }
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn filters_by_status_and_collection() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        seed(
            &store,
            &[
                ("00000001", "Inbox", TaskStatus::Ready),
                ("00000002", "Work/A", TaskStatus::Completed),
            ],
        );
        assert_eq!(
            store
                .items(Some(TaskStatus::Ready), None, &[], None)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            store.items(None, Some("Work/A"), &[], None).unwrap().len(),
            1
        );
    }

    #[test]
    fn resolves_id_by_unique_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        seed(&store, &[("0123abcd", "Inbox", TaskStatus::Ready)]);
        let found = store
            .items(None, None, &["0123".to_string()], None)
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "0123abcd");
    }

    #[test]
    fn add_trims_and_defaults_collection() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let item = store
            .add("  Buy milk  ", "", None, false, TaskStatus::Ready)
            .unwrap();
        assert_eq!(item.title, "Buy milk");
        assert_eq!(item.collection, "Inbox");
        assert!(is_valid_id(&item.id));
    }

    #[test]
    fn add_rejects_empty_title_unless_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        assert_eq!(
            store
                .add("   ", "Inbox", None, false, TaskStatus::Ready)
                .unwrap_err(),
            StoreError::InvalidTitle
        );
        let empty = store
            .add("", "Inbox", None, true, TaskStatus::Draft)
            .unwrap();
        assert_eq!(empty.title, "");
        assert_eq!(empty.status, TaskStatus::Draft);
    }

    #[test]
    fn add_rejects_duplicate_and_invalid_ids() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add("a", "Inbox", Some("0123abcd"), false, TaskStatus::Ready)
            .unwrap();
        assert_eq!(
            store
                .add("b", "Inbox", Some("0123abcd"), false, TaskStatus::Ready)
                .unwrap_err(),
            StoreError::DuplicateId("0123abcd".into())
        );
        assert_eq!(
            store
                .add("c", "Inbox", Some("xyz"), false, TaskStatus::Ready)
                .unwrap_err(),
            StoreError::InvalidId("xyz".into())
        );
    }

    #[test]
    fn update_bumps_version_and_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let created = store
            .add("a", "Inbox", Some("0123abcd"), false, TaskStatus::Ready)
            .unwrap();
        let updated = store.update("0123abcd", Some("a2"), None, None).unwrap();
        assert_eq!(updated.title, "a2");
        assert_ne!(updated.version, created.version);
    }

    #[test]
    fn update_requires_a_field() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add("a", "Inbox", Some("0123abcd"), false, TaskStatus::Ready)
            .unwrap();
        assert_eq!(
            store.update("0123abcd", None, None, None).unwrap_err(),
            StoreError::MissingUpdate
        );
    }

    #[test]
    fn if_current_skips_on_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let created = store
            .add("a", "Inbox", Some("0123abcd"), false, TaskStatus::Ready)
            .unwrap();
        store
            .update("0123abcd", Some("changed"), None, None)
            .unwrap(); // now stale
        let result = store
            .update_if_current("0123abcd", Some("x"), None, None, &created)
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn move_changes_collection() {
        let dir = tempfile::tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add("a", "Inbox", Some("0123abcd"), false, TaskStatus::Ready)
            .unwrap();
        let moved = store.move_item("0123abcd", "Work/A").unwrap();
        assert_eq!(moved.collection, "Work/A");
    }
}
