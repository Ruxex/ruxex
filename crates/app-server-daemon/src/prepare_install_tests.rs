//! Covers complete-package staging, conservative selection, and legacy preservation.
#![cfg(unix)]

use super::prepare_from_package;
use super::validate_package;
use crate::settings::DaemonSettings;
use pretty_assertions::assert_eq;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

fn daemon(home: &Path) -> crate::Daemon {
    let state = home.join("app-server-daemon");
    crate::Daemon {
        socket_path: state.join("app-server.sock"),
        pid_file: state.join("app-server.pid"),
        operation_lock_file: state.join("daemon.lock"),
        settings_file: state.join("settings.json"),
        managed_genex_bin: crate::managed_install::managed_genex_bin(home),
        host_command_args: None,
    }
}

fn package(root: &Path, version: &str) -> PathBuf {
    let target = super::platform_target().expect("target");
    for dir in ["bin", "genex-path", "genex-resources/nested"] {
        std::fs::create_dir_all(root.join(dir)).expect("package directory");
    }
    let bin = root.join("bin/genex");
    std::fs::write(&bin, format!("#!/bin/sh\necho 'genex {version}'\n")).expect("genex executable");
    for file in [
        "bin/genex-code-mode-host",
        "genex-path/rg",
        "genex-resources/nested/runtime",
    ] {
        std::fs::write(root.join(file), b"runtime").expect("package file");
        if file != "genex-resources/nested/runtime" {
            std::fs::set_permissions(root.join(file), std::fs::Permissions::from_mode(0o755))
                .expect("executable helper");
        }
    }
    if cfg!(target_os = "linux") {
        std::fs::write(root.join("genex-resources/bwrap"), b"runtime").expect("bwrap");
        std::fs::set_permissions(
            root.join("genex-resources/bwrap"),
            std::fs::Permissions::from_mode(0o755),
        )
        .expect("executable bwrap");
    }
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755))
        .expect("executable permission");
    std::fs::write(
        root.join("genex-package.json"),
        serde_json::json!({
            "version": version, "target": target, "entrypoint": "bin/genex"
        })
        .to_string(),
    )
    .expect("manifest");
    bin
}

#[tokio::test]
async fn seeds_full_package() {
    let temp = tempfile::TempDir::new().expect("temp");
    let home = temp.path().join("home");
    let old = temp.path().join("old");
    let old_bin = package(&old, "0.152.0");
    prepare_from_package(
        &daemon(&home),
        &DaemonSettings::default(),
        Some(&old),
        &old_bin,
    )
    .await
    .expect("seed");

    let root = home.join("packages/app-server-daemon");
    let selected = std::fs::canonicalize(root.join("current")).expect("selected");
    assert_eq!(
        std::fs::read(selected.join("genex-resources/nested/runtime")).expect("runtime"),
        b"runtime"
    );
    assert!(validate_package(&selected).is_ok());
}

#[tokio::test]
async fn incomplete_source_fails_without_selecting_it() {
    let temp = tempfile::TempDir::new().expect("temp");
    let source = temp.path().join("package");
    let bin = package(&source, "0.152.0");
    std::fs::remove_file(source.join("bin/genex-code-mode-host")).expect("remove helper");
    let home = temp.path().join("home");
    let error = prepare_from_package(
        &daemon(&home),
        &DaemonSettings::default(),
        Some(&source),
        &bin,
    )
    .await
    .expect_err("incomplete package");
    assert!(error.to_string().contains("bin/genex-code-mode-host"));
    assert!(!home.join("packages/app-server-daemon/current").exists());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn provisioned_macos_bundle_seeds_from_its_running_executable() {
    let temp = tempfile::TempDir::new().expect("temp");
    let source = temp.path().join("package");
    let launcher = package(&source, "0.1.0-internal-test.202609091200.1");
    std::fs::write(&launcher, b"#!/bin/sh\necho genex 0.0.0\n").expect("launcher");
    let bundle = source.join("GenexCLI.app/Contents/MacOS/codex");
    std::fs::create_dir_all(bundle.parent().expect("bundle parent")).expect("bundle dir");
    std::fs::write(&bundle, b"provisioned executable").expect("bundle executable");
    let home = temp.path().join("home");
    prepare_from_package(
        &daemon(&home),
        &DaemonSettings::default(),
        Some(&source),
        &bundle,
    )
    .await
    .expect("seed provisioned bundle");
    let selected = std::fs::canonicalize(home.join("packages/app-server-daemon/current"))
        .expect("selected release");
    assert_eq!(
        std::fs::read(selected.join("GenexCLI.app/Contents/MacOS/codex"))
            .expect("bundled executable"),
        b"provisioned executable"
    );
}

#[tokio::test]
async fn legacy_selection_is_not_migrated_or_refreshed() {
    let temp = tempfile::TempDir::new().expect("temp");
    let home = temp.path().join("home");
    let legacy = home.join("packages/standalone");
    let bin = package(&legacy.join("releases/old"), "0.150.0");
    std::os::unix::fs::symlink("releases/old", legacy.join("current")).expect("legacy current");
    let state = home.join("app-server-daemon");
    std::fs::create_dir(&state).expect("state");
    std::fs::write(state.join("app-server.stderr.log"), b"").expect("legacy marker");
    let source = temp.path().join("new");
    let new_bin = package(&source, "0.160.0");
    prepare_from_package(
        &daemon(&home),
        &DaemonSettings::default(),
        Some(&source),
        &new_bin,
    )
    .await
    .expect("preserve legacy selection");
    assert_eq!(
        crate::managed_install::managed_genex_bin(&home)
            .canonicalize()
            .expect("selected binary"),
        bin.canonicalize().expect("legacy binary")
    );
    assert!(!home.join("packages/app-server-daemon").exists());
}

#[tokio::test]
async fn broken_selection_is_not_a_missing_installation() {
    let temp = tempfile::TempDir::new().expect("temp");
    let home = temp.path().join("home");
    let source = temp.path().join("source");
    let bin = package(&source, "0.152.0");
    let daemon = daemon(&home);
    let current = home.join("packages/app-server-daemon/current");
    std::fs::create_dir_all(current.parent().expect("current parent")).expect("package root");
    std::os::unix::fs::symlink("missing-release", &current).expect("broken selection");
    let error = prepare_from_package(&daemon, &DaemonSettings::default(), Some(&source), &bin)
        .await
        .expect_err("broken installation");
    assert!(
        error
            .to_string()
            .contains("repair the existing installation")
    );
    assert_eq!(
        std::fs::read_link(current).expect("current link"),
        PathBuf::from("missing-release")
    );
}
