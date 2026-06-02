use directories::ProjectDirs;
use std::path::PathBuf;

const STORE_ENV: &str = "POND_STORE";

/// Directory for the data store and install record. macOS:
/// `~/Library/Application Support/pond/`, Linux: `~/.local/share/pond/`,
/// Windows: `%APPDATA%\pond\`.
pub fn data_directory() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("", "", "pond") {
        return dirs.data_dir().to_path_buf();
    }
    PathBuf::from("pond")
}

/// Resolved store path, honoring the `POND_STORE` override.
pub fn default_store_path() -> PathBuf {
    if let Ok(override_path) = std::env::var(STORE_ENV) {
        if !override_path.is_empty() {
            return PathBuf::from(override_path);
        }
    }
    data_directory().join("tasks.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_path_defaults_to_tasks_json() {
        // With the env unset, the file name is tasks.json under the data dir.
        // (Run serially elsewhere; here we only assert the suffix when unset.)
        if std::env::var(STORE_ENV).is_err() {
            assert!(default_store_path().ends_with("tasks.json"));
        }
    }

    #[test]
    fn data_directory_is_named_pond() {
        let dir = data_directory();
        assert_eq!(dir.file_name().unwrap().to_str().unwrap(), "pond");
    }
}
