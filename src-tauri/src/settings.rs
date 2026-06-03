use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// App-settings, persisted to `<config_dir>/pond/settings.json`. GUI-only
/// (`pond-tauri`-local); the CLI does not read these. `#[serde(default)]` on
/// every field means a partial or absent file loads with defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Editing a title drops the task to Draft; confirming promotes to Ready.
    pub uses_auto_draft: bool,
    /// Keep the window above other windows.
    pub always_on_top: bool,
    /// Default prompt template for new collections (used in 5B).
    pub default_prompt_template: String,
    /// The collection selected when the app was last closed (restored on launch).
    pub last_selected_collection: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            uses_auto_draft: true,
            always_on_top: false,
            default_prompt_template: String::new(),
            last_selected_collection: None,
        }
    }
}

/// `<config_dir>/pond/settings.json`. Same app identifiers as
/// `pond_core::paths` (`ProjectDirs::from("", "", "pond")`), but the config dir
/// rather than the data dir. Falls back to a relative `pond/settings.json`.
pub fn settings_path() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("", "", "pond") {
        return dirs.config_dir().join("settings.json");
    }
    PathBuf::from("pond").join("settings.json")
}

/// Load settings from `path`. A missing or corrupt file yields `Settings::default()`
/// (a fresh, valid file is written on the next `save`).
pub fn load(path: &Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Persist settings to `path` (pretty JSON, atomic temp+rename). Creates the
/// parent directory if needed.
pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json.as_bytes())?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn defaults_when_file_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = load(&path);
        assert_eq!(s, Settings::default());
        assert!(s.uses_auto_draft);
        assert!(!s.always_on_top);
        assert_eq!(s.default_prompt_template, "");
        assert_eq!(s.last_selected_collection, None);
    }

    #[test]
    fn partial_file_fills_missing_fields_with_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        // Only one field present; the rest must default.
        std::fs::write(&path, br#"{ "alwaysOnTop": true }"#).unwrap();
        let s = load(&path);
        assert!(s.always_on_top);
        assert!(s.uses_auto_draft); // default true survives
        assert_eq!(s.last_selected_collection, None);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, b"not json at all").unwrap();
        assert_eq!(load(&path), Settings::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let original = Settings {
            uses_auto_draft: false,
            always_on_top: true,
            default_prompt_template: "Plan: {{title}}".to_string(),
            last_selected_collection: Some("Work/Docs".to_string()),
        };
        save(&path, &original).unwrap();
        assert_eq!(load(&path), original);
    }

    #[test]
    fn save_overwrites_persisted_value() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        save(&path, &Settings::default()).unwrap();
        let updated = Settings {
            last_selected_collection: Some("Inbox".to_string()),
            ..Settings::default()
        };
        save(&path, &updated).unwrap();
        let loaded = load(&path);
        assert_eq!(loaded.last_selected_collection, Some("Inbox".to_string()));
        // No leftover temp file.
        assert!(!path.with_extension("json.tmp").exists());
    }
}
