use crate::document::TaskFile;
use crate::error::{Result, StoreError};
use crate::json::to_pretty_sorted;
use fs2::FileExt;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

pub struct TaskStore {
    file_path: PathBuf,
    lock_path: PathBuf,
}

#[allow(dead_code)]
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
}
