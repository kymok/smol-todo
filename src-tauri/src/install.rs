//! CLI-install IPC support (unix-only; mirrors pond_core::cli_install, which is #![cfg(unix)]).
//! `target_beside`/`resolve_taskpond_target` pick the install target (the binary beside the
//! app executable); `InstallStatusDto` + `dto_from` map pond-core's `InstallStatus` to a
//! camelCase wire DTO (adding the derived `canInstall` and the `Installer`-level `pathHint`).
use pond_core::cli_install::InstallStatus;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// The install target sitting beside a given executable path: `<exe-dir>/taskpond`.
/// Pure (no `current_exe`) so it is unit-testable.
pub fn target_beside(exe: &Path) -> PathBuf {
    exe.parent()
        .unwrap_or_else(|| Path::new(""))
        .join("taskpond")
}

/// The real install target: `taskpond` next to the current executable. `None` when the
/// current exe / its parent cannot be resolved (caller falls back to a bare "taskpond").
pub fn resolve_taskpond_target() -> Option<PathBuf> {
    Some(target_beside(&std::env::current_exe().ok()?))
}

/// camelCase wire mirror of `InstallStatus` + the derived `canInstall` + `pathHint`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallStatusDto {
    pub link_path: String,
    pub target_path: String,
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_description: Option<String>,
    pub install_directory_is_in_path: bool,
    pub can_uninstall: bool,
    pub can_install: bool,
    pub path_hint: String,
}

/// Build the DTO from an `InstallStatus` plus the `Installer`-level `path_hint`
/// (`path_hint` is NOT a field of `InstallStatus`, and `can_install` is a method).
pub fn dto_from(status: &InstallStatus, path_hint: String) -> InstallStatusDto {
    InstallStatusDto {
        link_path: status.link_path.to_string_lossy().to_string(),
        target_path: status.target_path.to_string_lossy().to_string(),
        installed: status.installed,
        conflict_description: status.conflict_description.clone(),
        install_directory_is_in_path: status.install_directory_is_in_path,
        can_uninstall: status.can_uninstall,
        can_install: status.can_install(),
        path_hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pond_core::cli_install::Installer;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use tempfile::tempdir;

    fn make_executable(path: &Path) {
        fs::write(path, b"#!/bin/sh\n").unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn target_beside_joins_taskpond_to_parent() {
        assert_eq!(
            target_beside(Path::new("/Applications/Pond.app/Contents/MacOS/Pond")),
            PathBuf::from("/Applications/Pond.app/Contents/MacOS/taskpond")
        );
        // No parent → just "taskpond".
        assert_eq!(target_beside(Path::new("Pond")), PathBuf::from("taskpond"));
    }

    #[test]
    fn dto_from_round_trips_fields_and_derives_can_install() {
        // A not-installed, no-conflict status → can_install == true (per InstallStatus::can_install).
        let status = InstallStatus {
            link_path: PathBuf::from("/home/u/.local/bin/taskpond"),
            target_path: PathBuf::from("/app/taskpond"),
            installed: false,
            conflict_description: None,
            install_directory_is_in_path: false,
            can_uninstall: false,
        };
        let dto = dto_from(&status, "EXPORT".to_string());
        assert_eq!(dto.link_path, "/home/u/.local/bin/taskpond");
        assert_eq!(dto.target_path, "/app/taskpond");
        assert!(!dto.installed);
        assert_eq!(dto.conflict_description, None);
        assert!(!dto.install_directory_is_in_path);
        assert!(!dto.can_uninstall);
        assert!(dto.can_install); // derived from the method
        assert_eq!(dto.path_hint, "EXPORT");
    }

    #[test]
    fn dto_from_installed_status_cannot_install() {
        let status = InstallStatus {
            link_path: PathBuf::from("/l"),
            target_path: PathBuf::from("/t"),
            installed: true,
            conflict_description: None,
            install_directory_is_in_path: true,
            can_uninstall: true,
        };
        let dto = dto_from(&status, String::new());
        assert!(dto.installed);
        assert!(!dto.can_install); // installed → cannot install
        assert!(dto.can_uninstall);
    }

    #[test]
    fn dto_installed_flag_flips_across_install_uninstall() {
        // Build an Installer on tempdir paths with an executable target, then drive
        // status -> install -> status -> uninstall -> status through the DTO mapping.
        let dir = tempdir().unwrap();
        let target = dir.path().join("taskpond-bin");
        make_executable(&target);
        let link = dir.path().join("bin/taskpond");
        let record = dir.path().join("cli-install.json");
        let installer = Installer::new(link, target, record);

        let before = dto_from(&installer.status(), installer.path_hint());
        assert!(!before.installed);

        installer.install().unwrap();
        let after_install = dto_from(&installer.status(), installer.path_hint());
        assert!(after_install.installed);
        assert!(!after_install.can_install);

        installer.uninstall().unwrap();
        let after_uninstall = dto_from(&installer.status(), installer.path_hint());
        assert!(!after_uninstall.installed);
    }
}
