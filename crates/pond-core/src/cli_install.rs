//! Command-line installer: symlinks `~/.local/bin/taskpond` at the bundled binary.
//! Unix-only (macOS-primary); port of the Swift `CommandLineInstaller`.
#![cfg(unix)]

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

    pub fn install(&self) -> Result<()> {
        if !Self::is_executable(&self.target_path) {
            return Err(StoreError::Io(format!(
                "CLI executable was not found at {}.",
                self.target_path.display()
            )));
        }
        if let Some(parent) = self.link_path.parent() {
            fs::create_dir_all(parent)?;
        }
        match link_kind(&self.link_path) {
            LinkKind::Missing => {
                std::os::unix::fs::symlink(&self.target_path, &self.link_path)?;
                self.write_record()
            }
            LinkKind::File => Err(StoreError::Io(format!(
                "{} already exists and is not a symlink created by Pond.",
                self.link_path.display()
            ))),
            LinkKind::Symlink(dest) => {
                if dest == self.target_path {
                    self.write_record()
                } else if self.can_remove_symlink(&dest) {
                    fs::remove_file(&self.link_path)?;
                    std::os::unix::fs::symlink(&self.target_path, &self.link_path)?;
                    self.write_record()
                } else {
                    Err(StoreError::Io(format!(
                        "{} already points to {}.",
                        self.link_path.display(),
                        dest.display()
                    )))
                }
            }
        }
    }

    pub fn uninstall(&self) -> Result<()> {
        match link_kind(&self.link_path) {
            LinkKind::Missing => {
                let _ = fs::remove_file(&self.record_path);
                Ok(())
            }
            LinkKind::File => Err(StoreError::Io(format!(
                "{} already exists and is not a symlink created by Pond.",
                self.link_path.display()
            ))),
            LinkKind::Symlink(dest) => {
                if self.can_remove_symlink(&dest) {
                    fs::remove_file(&self.link_path)?;
                    let _ = fs::remove_file(&self.record_path);
                    Ok(())
                } else {
                    Err(StoreError::Io(format!(
                        "{} already points to {}.",
                        self.link_path.display(),
                        dest.display()
                    )))
                }
            }
        }
    }

    fn write_record(&self) -> Result<()> {
        if let Some(parent) = self.record_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let record = InstallRecord {
            link_path: self.link_path.to_string_lossy().to_string(),
            target_path: self.target_path.to_string_lossy().to_string(),
            installed_at: chrono::Utc::now(),
        };
        let json = crate::json::to_pretty_sorted(&record)?;
        // Atomic write (temp + rename), matching the store's write_file and Swift's .atomic,
        // so a crash mid-write cannot leave a corrupt record file.
        let mut tmp = self.record_path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp_path = PathBuf::from(tmp);
        fs::write(&tmp_path, json.as_bytes())?;
        fs::rename(&tmp_path, &self.record_path)?;
        Ok(())
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

    #[test]
    fn install_then_uninstall_round_trip() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("taskpond-bin");
        make_executable(&target);
        let link = dir.path().join("bin/taskpond");
        let record = dir.path().join("cli-install.json");
        let installer = Installer::new(link.clone(), target.clone(), record.clone());

        installer.install().unwrap();
        assert!(installer.status().installed);
        assert!(record.exists());

        installer.uninstall().unwrap();
        assert!(!installer.status().installed);
        assert!(!record.exists());
        assert!(fs::symlink_metadata(&link).is_err()); // link removed
    }

    #[test]
    fn install_rejects_missing_executable() {
        let dir = tempdir().unwrap();
        let installer = Installer::new(
            dir.path().join("link"),
            dir.path().join("does-not-exist"),
            dir.path().join("rec.json"),
        );
        let err = installer.install().unwrap_err();
        assert!(format!("{err}").contains("was not found"));
    }

    #[test]
    fn install_refuses_to_clobber_foreign_file() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("taskpond-bin");
        make_executable(&target);
        let link = dir.path().join("link");
        fs::write(&link, b"i am a real file").unwrap();
        let installer = Installer::new(link, target, dir.path().join("rec.json"));
        let err = installer.install().unwrap_err();
        assert!(format!("{err}").contains("not a symlink created by Pond"));
    }
}
