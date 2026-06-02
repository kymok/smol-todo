//! Command-line installer: symlinks `~/.local/bin/taskpond` at the bundled binary.
//! Unix-only (macOS-primary); port of the Swift `CommandLineInstaller`.
#![cfg(unix)]
// `Result` and `is_executable` are used by Task 11; allow until then.
#![allow(unused_imports, dead_code)]

use crate::error::{Result, StoreError};
use crate::paths::data_directory;
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallStatus {
    pub link_path: PathBuf,
    pub target_path: PathBuf,
    pub installed: bool,
    pub conflict_description: Option<String>,
    pub install_directory_is_in_path: bool,
    pub can_uninstall: bool,
}

impl InstallStatus {
    pub fn can_install(&self) -> bool {
        !self.installed && (self.conflict_description.is_none() || self.can_uninstall)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallRecord {
    link_path: String,
    target_path: String,
    installed_at: chrono::DateTime<chrono::Utc>,
}

pub struct Installer {
    pub link_path: PathBuf,
    pub target_path: PathBuf,
    pub record_path: PathBuf,
}

enum LinkKind {
    Missing,
    File,
    Symlink(PathBuf),
}

fn link_kind(path: &Path) -> LinkKind {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return LinkKind::Missing,
    };
    if meta.file_type().is_symlink() {
        match fs::read_link(path) {
            Ok(dest) => {
                let resolved = if dest.is_absolute() {
                    dest
                } else {
                    path.parent().unwrap_or(Path::new("")).join(dest)
                };
                LinkKind::Symlink(resolved)
            }
            Err(_) => LinkKind::File,
        }
    } else {
        LinkKind::File
    }
}

fn path_contains(dir: &Path) -> bool {
    let target = dir.to_string_lossy();
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .any(|entry| entry == target)
}

impl Installer {
    /// Construct with explicit paths (used by tests and the GUI).
    pub fn new(link_path: PathBuf, target_path: PathBuf, record_path: PathBuf) -> Self {
        Installer {
            link_path,
            target_path,
            record_path,
        }
    }

    /// Default link (`~/.local/bin/taskpond`) and record (`<data-dir>/cli-install.json`).
    /// The target defaults to the current executable; packaging finalizes this for the
    /// Tauri bundle in a later phase.
    pub fn with_defaults() -> Self {
        let home = directories::BaseDirs::new()
            .map(|b| b.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let target = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("taskpond"));
        Installer {
            link_path: home.join(".local/bin/taskpond"),
            target_path: target,
            record_path: data_directory().join("cli-install.json"),
        }
    }

    pub fn path_hint(&self) -> String {
        r#"export PATH="$HOME/.local/bin:$PATH""#.to_string()
    }

    pub fn status(&self) -> InstallStatus {
        let in_path = self.link_path.parent().map(path_contains).unwrap_or(false);
        match link_kind(&self.link_path) {
            LinkKind::Missing => InstallStatus {
                link_path: self.link_path.clone(),
                target_path: self.target_path.clone(),
                installed: false,
                conflict_description: None,
                install_directory_is_in_path: in_path,
                can_uninstall: false,
            },
            LinkKind::File => InstallStatus {
                link_path: self.link_path.clone(),
                target_path: self.target_path.clone(),
                installed: false,
                conflict_description: Some(format!("{} already exists.", self.link_path.display())),
                install_directory_is_in_path: in_path,
                can_uninstall: false,
            },
            LinkKind::Symlink(dest) => {
                let installed = dest == self.target_path;
                InstallStatus {
                    link_path: self.link_path.clone(),
                    target_path: self.target_path.clone(),
                    installed,
                    conflict_description: if installed {
                        None
                    } else {
                        Some(format!(
                            "{} points to {}.",
                            self.link_path.display(),
                            dest.display()
                        ))
                    },
                    install_directory_is_in_path: in_path,
                    can_uninstall: self.can_remove_symlink(&dest),
                }
            }
        }
    }

    fn can_remove_symlink(&self, dest: &Path) -> bool {
        if dest == self.target_path {
            return true;
        }
        match self.read_record() {
            Some(record) => Path::new(&record.target_path) == dest,
            None => false,
        }
    }

    fn read_record(&self) -> Option<InstallRecord> {
        let data = fs::read(&self.record_path).ok()?;
        serde_json::from_slice(&data).ok()
    }

    pub(crate) fn is_executable(path: &Path) -> bool {
        fs::metadata(path)
            .map(|m| m.is_file() && (m.permissions().mode() & 0o111 != 0))
            .unwrap_or(false)
    }

    pub(crate) fn _unused(&self) {
        let _ = StoreError::InvalidTitle; // keep StoreError import until Task 11 uses it
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    fn make_executable(path: &Path) {
        fs::write(path, b"#!/bin/sh\n").unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn status_missing_then_installed() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("taskpond-bin");
        make_executable(&target);
        let link = dir.path().join("link");
        let record = dir.path().join("cli-install.json");
        let installer = Installer::new(link.clone(), target.clone(), record);

        let status = installer.status();
        assert!(!status.installed);
        assert!(status.conflict_description.is_none());

        symlink(&target, &link).unwrap();
        let status = installer.status();
        assert!(status.installed);
        assert!(status.can_install() == false);
    }

    #[test]
    fn status_conflict_for_foreign_symlink() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("taskpond-bin");
        make_executable(&target);
        let other = dir.path().join("other-bin");
        make_executable(&other);
        let link = dir.path().join("link");
        symlink(&other, &link).unwrap();
        let installer = Installer::new(link, target, dir.path().join("rec.json"));
        let status = installer.status();
        assert!(!status.installed);
        assert!(status
            .conflict_description
            .as_ref()
            .unwrap()
            .contains("points to"));
        assert!(!status.can_uninstall); // foreign symlink, no record
    }
}
