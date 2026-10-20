//! Installs a complete CLI package when the managed daemon is missing.

use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use ruxex_install_context::InstallContext;
use ruxex_install_context::RuxexPackageManifest;

use crate::Daemon;
use crate::install_lock::acquire_install_lock;
use crate::managed_install;
use crate::settings::DaemonSettings;

/// Prepare a missing package while the caller holds the daemon operation lock.
pub(super) async fn prepare(
    daemon: &Daemon,
    settings: &DaemonSettings,
    package_source: Option<&Path>,
) -> Result<()> {
    let source = package_source.map(Path::to_path_buf).or_else(|| {
        InstallContext::current()
            .package_layout
            .as_ref()
            .map(|layout| layout.package_dir.clone().into_path_buf())
    });
    Box::pin(prepare_from_package(
        daemon,
        settings,
        source.as_deref(),
        &std::env::current_exe()?,
    ))
    .await
    .map(|_| ())
}

async fn prepare_from_package(
    daemon: &Daemon,
    settings: &DaemonSettings,
    source: Option<&Path>,
    running_exe: &Path,
) -> Result<bool> {
    let home = daemon
        .settings_file
        .parent()
        .and_then(Path::parent)
        .context("daemon settings path has no Ruxex home")?;
    let previous_root = managed_install::package_root(home);
    let root = home.join("packages/app-server-daemon");
    anyhow::ensure!(
        daemon.managed_ruxex_bin.starts_with(&previous_root),
        "daemon package location changed; retry the command"
    );
    if previous_root != root
        || daemon.running_backend_instance(settings).await?.is_some()
        || crate::client::probe(&daemon.socket_path).await.is_ok()
    {
        return Ok(true);
    }
    if !matches!(root.join("current").symlink_metadata(), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
        || ["daemon.pid", "daemon.stderr.log"].iter().any(|name| {
            !matches!(
                home.join("app-server-daemon").join(name).symlink_metadata(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound
            )
        })
    {
        daemon.ensure_managed_ruxex_bin()?;
        return Ok(true);
    }
    std::fs::create_dir_all(&root)?;
    anyhow::ensure!(
        managed_install::package_root(home) == previous_root,
        "daemon package location changed; retry the command"
    );
    let backend = daemon.running_backend_instance(settings).await?;
    anyhow::ensure!(
        backend.is_some() || crate::client::probe(&daemon.socket_path).await.is_err(),
        "app server is running but is not managed by ruxex app-server daemon"
    );
    let selected = managed_install::managed_ruxex_bin(home);
    let previous_release = previous_root.join("current").canonicalize().ok();
    if selected.is_file() {
        return Ok(true);
    }
    anyhow::ensure!(
        backend.is_none()
            && matches!(root.join("current").symlink_metadata(), Err(error) if error.kind() == std::io::ErrorKind::NotFound),
        "the selected daemon package is incomplete; repair its installation before starting"
    );
    let source = source.context(
        "this CLI has no complete local package; install a packaged Ruxex CLI or use the standalone installer",
    )?;
    anyhow::ensure!(
        !root.canonicalize()?.starts_with(source.canonicalize()?),
        "RUXEX_HOME must be outside the source CLI package"
    );
    let manifest_bytes = std::fs::read(source.join("ruxex-package.json"))?;
    let manifest: RuxexPackageManifest = serde_json::from_slice(&manifest_bytes)?;
    let version = manifest.version.to_string();
    let target = platform_target()?;
    let metadata: serde_json::Value = serde_json::from_slice(&manifest_bytes)?;
    let entrypoint = "bin/ruxex";
    anyhow::ensure!(
        metadata["target"] == target && metadata["entrypoint"] == entrypoint,
        "the CLI package does not match this platform or executable"
    );
    validate_package(source)?;
    // A local build may replace the executable while package preparation is in progress.
    let running_identity = managed_install::executable_identity(running_exe).await?;
    eprintln!(
        "Installing daemon from CLI version {version} into {}...",
        root.display()
    );
    // Initial startup already holds the lifecycle operation lock.
    let _install_lock = acquire_install_lock(&root).await?;
    anyhow::ensure!(
        managed_install::package_root(home) == previous_root,
        "daemon package location changed; retry the command"
    );
    if managed_install::managed_ruxex_bin(home).is_file() {
        return Ok(true);
    }
    anyhow::ensure!(
        previous_root.join("current").canonicalize().ok() == previous_release,
        "daemon selection changed while awaiting confirmation; retry the command"
    );
    let current_backend = daemon.running_backend_instance(settings).await?;
    anyhow::ensure!(
        current_backend.is_some() == backend.is_some(),
        "daemon running state changed while awaiting confirmation; retry the command"
    );
    let backend = current_backend;
    let stable = stable_version(&version).is_some();
    let releases = root.join("releases");
    std::fs::create_dir_all(&releases)?;
    let stage = tempfile::Builder::new()
        .prefix(".staging.")
        .tempdir_in(&releases)?;
    let digest = package_tree(source, Some(stage.path()))?;
    validate_package(stage.path())?;
    let staged_exe = if cfg!(target_os = "macos") && stage.path().join("RuxexCLI.app").is_dir() {
        stage.path().join("RuxexCLI.app/Contents/MacOS/codex")
    } else {
        stage.path().join(entrypoint)
    };
    anyhow::ensure!(
        package_tree(source, /*destination*/ None)? == digest
            && std::fs::read(stage.path().join("ruxex-package.json"))? == manifest_bytes
            && managed_install::executable_identity(&staged_exe).await? == running_identity,
        "the CLI package changed while preparing the daemon or differs from the running executable"
    );
    let binary_version =
        managed_install::managed_ruxex_version(&stage.path().join(entrypoint)).await?;
    anyhow::ensure!(
        !stable || version == binary_version,
        "the CLI package version does not match its executable"
    );
    let name = if stable {
        format!("{version}-{target}")
    } else {
        format!("local-{digest}-{target}")
    };
    let release = releases.join(&name);
    if release.try_exists()? {
        anyhow::ensure!(
            !release.symlink_metadata()?.file_type().is_symlink()
                && package_tree(&release, /*destination*/ None)? == digest,
            "an existing daemon release has different contents; refusing to overwrite it"
        );
    } else {
        #[cfg(unix)]
        if !stage.path().join("ruxex").exists() {
            std::os::unix::fs::symlink("bin/ruxex", stage.path().join("ruxex"))?;
        }
        std::fs::rename(stage.path(), &release)?;
    }
    anyhow::ensure!(
        managed_install::package_root(home) == previous_root
            && (backend.is_some() || crate::client::probe(&daemon.socket_path).await.is_err()),
        "daemon package location or socket ownership changed while preparing its package; retry the command"
    );
    #[cfg(unix)]
    {
        let temporary = tempfile::TempDir::new_in(&root)?;
        let link = temporary.path().join("current");
        std::os::unix::fs::symlink(&release, &link)?;
        std::fs::rename(link, root.join("current"))?;
    }
    Ok(true)
}

/// Hash the complete tree and optionally copy those same bytes. Relative file
/// links are materialized; escaping links and directory links are rejected.
fn package_tree(root: &Path, destination: Option<&Path>) -> Result<String> {
    let canonical_root = root.canonicalize()?;
    let mut hasher = blake3::Hasher::new();
    let mut paths = vec![root.to_path_buf()];
    while let Some(path) = paths.pop() {
        let relative = path.strip_prefix(root)?;
        // The Unix installer adds this alias outside the package layout.
        if cfg!(unix)
            && relative == Path::new("codex")
            && std::fs::read_link(&path).ok().as_deref() == Some(Path::new("bin/ruxex"))
        {
            continue;
        }
        anyhow::ensure!(
            path.canonicalize()?.starts_with(&canonical_root),
            "package link escapes its root"
        );
        let metadata = path.metadata()?;
        hasher.update(relative.as_os_str().as_encoded_bytes());
        hasher.update(&[0]);
        if metadata.is_dir() {
            anyhow::ensure!(
                !path.symlink_metadata()?.file_type().is_symlink(),
                "package contains a directory link"
            );
            hasher.update(b"directory");
            if let Some(destination) = destination
                && !relative.as_os_str().is_empty()
            {
                std::fs::create_dir(destination.join(relative))?;
            }
            let mut entries = std::fs::read_dir(&path)?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<std::io::Result<Vec<_>>>()?;
            entries.sort();
            paths.extend(entries);
        } else {
            anyhow::ensure!(metadata.is_file(), "package contains an unsupported file");
            let bytes = std::fs::read(&path)?;
            hasher.update(b"file");
            hasher.update(blake3::hash(&bytes).as_bytes());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                hasher.update(&(metadata.permissions().mode() & 0o777).to_le_bytes());
            }
            if let Some(destination) = destination {
                let target = destination.join(relative);
                std::fs::write(&target, bytes)?;
                std::fs::set_permissions(target, metadata.permissions())?;
            }
        }
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn stable_version(value: &str) -> Option<semver::Version> {
    let version = semver::Version::parse(value).ok()?;
    (version.pre.is_empty()
        && version.build.is_empty()
        && (version.major, version.minor, version.patch) != (0, 0, 0))
        .then_some(version)
}

fn validate_package(root: &Path) -> Result<()> {
    let mut names = vec![
        "ruxex-package.json",
        "bin/ruxex",
        "bin/ruxex-code-mode-host",
        "ruxex-path/rg",
    ];
    if cfg!(target_os = "linux") {
        names.push("ruxex-resources/bwrap");
    }
    for name in names {
        if !root.join(name).is_file() {
            return Err(anyhow!(
                "local Ruxex package is missing {name}; reinstall the CLI or use the standalone installer"
            ));
        }
        #[cfg(unix)]
        if name != "ruxex-package.json" {
            use std::os::unix::fs::PermissionsExt;
            if std::fs::metadata(root.join(name))?.permissions().mode() & 0o111 == 0 {
                return Err(anyhow!(
                    "local Ruxex package file {name} is not executable; reinstall the CLI or use the standalone installer"
                ));
            }
        }
    }
    Ok(())
}

fn platform_target() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("linux", "aarch64") if cfg!(target_env = "gnu") => Ok("aarch64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-musl"),
        ("linux", "x86_64") if cfg!(target_env = "gnu") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-musl"),
        (os, arch) => Err(anyhow!("unsupported packaged daemon platform {os}/{arch}")),
    }
}

#[cfg(test)]
#[path = "prepare_install_tests.rs"]
mod tests;
