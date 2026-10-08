//! Resolves both package and legacy standalone layouts and compares installed executables.

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use serde::Deserialize;
use serde::Serialize;
use tokio::fs;
use tokio::process::Command;

/// New daemons own their packages, regardless of how the calling CLI was installed.
/// Preserve legacy launch state, including logs left after a daemon is stopped;
/// settings, installer selections, and lock files alone do not prove a prior launch.
pub(crate) fn package_root(genex_home: &Path) -> PathBuf {
    let dedicated = genex_home.join("packages/app-server-daemon");
    if !matches!(dedicated.join("current").symlink_metadata(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound)
    {
        return dedicated;
    }
    let state = genex_home.join("app-server-daemon");
    for (package, artifacts) in [
        (
            "app-server-daemon",
            [
                crate::DAEMON_PID_FILE_NAME,
                "daemon.stderr.log",
            ],
        ),
        (
            "standalone",
            [
                crate::LEGACY_PID_FILE_NAME,
                "app-server.stderr.log",
            ],
        ),
    ] {
        if artifacts.iter().any(|name| {
            !matches!(state.join(name).symlink_metadata(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound)
        }) {
            return genex_home.join("packages").join(package);
        }
    }
    dedicated
}

/// Resolve both packaged and legacy binaries without requiring a valid install.
pub(crate) fn managed_genex_bin(genex_home: &Path) -> PathBuf {
    let root = package_root(genex_home);
    let current = root.join("current");
    let packaged = current.join("bin").join(managed_genex_file_name());
    let legacy = current.join("codex");
    if packaged.is_file() || !legacy.is_file() && root.ends_with("app-server-daemon") {
        packaged
    } else {
        legacy
    }
}

pub(crate) async fn managed_genex_version(genex_bin: &Path) -> Result<String> {
    let mut command = Command::new(genex_bin);
    let output = command
        .arg("--version")
        .kill_on_drop(true)
        .output()
        .await
        .with_context(|| {
            format!(
                "failed to invoke managed Genex binary {}",
                genex_bin.display()
            )
        })?;
    if !output.status.success() {
        return Err(anyhow!(
            "managed Genex binary {} exited with status {}",
            genex_bin.display(),
            output.status
        ));
    }

    let stdout = String::from_utf8(output.stdout).with_context(|| {
        format!(
            "managed Genex version was not utf-8: {}",
            genex_bin.display()
        )
    })?;
    parse_genex_version(&stdout)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ExecutableIdentity {
    digest: [u8; 32],
}

pub(crate) async fn executable_identity(executable: &Path) -> Result<ExecutableIdentity> {
    let bytes = fs::read(executable)
        .await
        .with_context(|| format!("failed to read executable {}", executable.display()))?;
    Ok(executable_identity_from_bytes(&bytes))
}

pub(crate) fn executable_identity_from_bytes(bytes: &[u8]) -> ExecutableIdentity {
    ExecutableIdentity {
        digest: *blake3::hash(bytes).as_bytes(),
    }
}

fn managed_genex_file_name() -> &'static str {
    "genex"
}

fn parse_genex_version(output: &str) -> Result<String> {
    let version = output
        .split_whitespace()
        .nth(1)
        .filter(|version| !version.is_empty())
        .ok_or_else(|| anyhow!("managed Genex version output was malformed"))?;
    Ok(version.to_string())
}

#[cfg(test)]
#[path = "managed_install_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "managed_install_path_tests.rs"]
mod path_tests;
