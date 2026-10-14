//! Managed app-server lifecycle, serialized across CLI invocations.

mod backend;
mod client;
mod install_lock;
mod managed_install;
mod prepare_install;
mod settings;
pub mod telemetry;
mod thread_recovery;

use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
pub use backend::BackendKind;
use backend::BackendPaths;
use managed_install::managed_ruxex_bin;
use managed_install::managed_ruxex_version;
use ruxex_app_server_transport::app_server_control_socket_path;
use ruxex_utils_home_dir::find_ruxex_home;
use serde::Serialize;
use settings::DaemonSettings;
use settings::MAX_SHUTDOWN_GRACE_SECONDS;
use tokio::time::sleep;

const START_POLL_INTERVAL: Duration = Duration::from_millis(50);
const START_TIMEOUT: Duration = Duration::from_secs(10);
// Leave room for the longest graceful stop, forced-exit check, and restart.
const OPERATION_LOCK_TIMEOUT: Duration =
    Duration::from_secs(MAX_SHUTDOWN_GRACE_SECONDS as u64 + 75);
const LEGACY_PID_FILE_NAME: &str = "app-server.pid";
const DAEMON_PID_FILE_NAME: &str = "daemon.pid";
const OPERATION_LOCK_FILE_NAME: &str = "daemon.lock";
const SETTINGS_FILE_NAME: &str = "settings.json";
const STATE_DIR_NAME: &str = "app-server-daemon";

pub const HOST_APP_SERVER_DAEMON_MODE_ARG: &str = "__ruxex-app-server-daemon";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleCommand {
    Start,
    Restart,
    Stop,
    Version,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LifecycleStatus {
    AlreadyRunning,
    Started,
    Restarted,
    Stopped,
    NotRunning,
    Running,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleOutput {
    pub status: LifecycleStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<BackendKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub managed_ruxex_path: PathBuf,
    pub managed_ruxex_version: Option<String>,
    pub socket_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_server_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapOptions {}

/// Passively probes an existing app-server socket and returns its reported
/// app-server version.
pub async fn probe_app_server_version(socket_path: &Path) -> Result<String> {
    Ok(client::probe(socket_path).await?.app_server_version)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BootstrapStatus {
    Bootstrapped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapOutput {
    pub status: BootstrapStatus,
    pub backend: BackendKind,
    pub managed_ruxex_path: PathBuf,
    pub managed_ruxex_version: Option<String>,
    pub socket_path: PathBuf,
    pub cli_version: String,
    pub app_server_version: String,
}

pub async fn run(command: LifecycleCommand) -> Result<LifecycleOutput> {
    ensure_supported_platform()?;
    // Keep daemon package preparation off callers' async stack frames.
    Box::pin(Daemon::from_environment()?.run(command)).await
}

/// Runs the daemon lifecycle against a host-owned executable.
///
/// The executable is expected to enter its AppServer daemon mode when invoked
/// with `command_args`. This lets a generic host launch the same binary that
/// owns its concrete provider type instead of launching the built-in managed
/// executable.
pub async fn run_with_host_executable(
    command: LifecycleCommand,
    executable: &Path,
    command_args: &[OsString],
) -> Result<LifecycleOutput> {
    ensure_supported_platform()?;
    Box::pin(
        Daemon::from_environment_with_host_executable(executable, command_args.to_vec())?
            .run(command),
    )
    .await
}

pub async fn bootstrap(options: BootstrapOptions) -> Result<BootstrapOutput> {
    ensure_supported_platform()?;
    Box::pin(Daemon::from_environment()?.bootstrap(options, None)).await
}

/// Bootstraps the managed app-server using an explicitly supplied complete
/// Ruxex package. Desktop hosts use this entry point because their executable
/// lives inside an application bundle rather than directly under `bin/`.
pub async fn bootstrap_from_package(package_source: &Path) -> Result<BootstrapOutput> {
    ensure_supported_platform()?;
    Box::pin(Daemon::from_environment()?.bootstrap(BootstrapOptions {}, Some(package_source))).await
}

/// Bootstraps a packaged provider-owned host and starts its hidden daemon mode.
///
/// Package preparation still uses the normal managed package layout. The
/// supplied arguments are used only when the installed host executable is
/// launched, so the package can own a concrete provider without exposing an
/// `app-server` subcommand or serializing a provider instance.
pub async fn bootstrap_from_package_with_host_args(
    package_source: &Path,
    host_command_args: &[OsString],
) -> Result<BootstrapOutput> {
    ensure_supported_platform()?;
    Box::pin(Daemon::from_environment()?.bootstrap_with_host_args(
        BootstrapOptions {},
        package_source,
        host_command_args,
    ))
    .await
}

#[cfg(unix)]
fn ensure_supported_platform() -> Result<()> {
    Ok(())
}

#[derive(Clone)]
struct Daemon {
    socket_path: PathBuf,
    pid_file: PathBuf,
    operation_lock_file: PathBuf,
    settings_file: PathBuf,
    managed_ruxex_bin: PathBuf,
    host_command_args: Option<Vec<OsString>>,
}

impl Daemon {
    fn from_environment() -> Result<Self> {
        let ruxex_home = find_ruxex_home().context("failed to resolve RUXEX_HOME")?;
        let socket_path = app_server_control_socket_path(ruxex_home.as_path())?
            .as_path()
            .to_path_buf();
        let state_dir = ruxex_home.as_path().join(STATE_DIR_NAME);
        let managed_ruxex_bin = managed_ruxex_bin(ruxex_home.as_path());
        // Old CLIs must not mistake a daemon-owned installation for their backend.
        let pid_file =
            if managed_ruxex_bin.starts_with(ruxex_home.as_path().join("packages/standalone")) {
                LEGACY_PID_FILE_NAME
            } else {
                DAEMON_PID_FILE_NAME
            };
        Ok(Self {
            socket_path,
            pid_file: state_dir.join(pid_file),
            operation_lock_file: state_dir.join(OPERATION_LOCK_FILE_NAME),
            settings_file: state_dir.join(SETTINGS_FILE_NAME),
            managed_ruxex_bin,
            host_command_args: None,
        })
    }

    fn from_environment_with_host_executable(
        executable: &Path,
        command_args: Vec<OsString>,
    ) -> Result<Self> {
        let mut daemon = Self::from_environment()?;
        daemon.managed_ruxex_bin = executable.to_path_buf();
        daemon.host_command_args = Some(command_args);
        Ok(daemon)
    }

    fn recovery_file(&self) -> Result<PathBuf> {
        Ok(ruxex_app_server_transport::daemon_recovery_file_path(
            self.settings_file
                .parent()
                .and_then(Path::parent)
                .context("daemon settings path has no Ruxex home")?,
        ))
    }

    // Call only after taking the operation lock: package selection may have
    // changed while this command was waiting.
    fn current_installation(&self) -> Result<Self> {
        let managed_ruxex_bin = self.current_managed_ruxex_bin()?;
        let home = self
            .settings_file
            .parent()
            .and_then(Path::parent)
            .context("daemon settings path has no Ruxex home")?;
        let pid = if managed_ruxex_bin.starts_with(home.join("packages/standalone")) {
            LEGACY_PID_FILE_NAME
        } else {
            DAEMON_PID_FILE_NAME
        };
        Ok(Self {
            managed_ruxex_bin,
            pid_file: self.pid_file.with_file_name(pid),
            ..self.clone()
        })
    }

    async fn run(&self, command: LifecycleCommand) -> Result<LifecycleOutput> {
        if command == LifecycleCommand::Version {
            return self.version().await;
        }
        let _operation_lock = self.acquire_operation_lock().await?;
        let selected = self.current_installation()?;
        match command {
            LifecycleCommand::Start => selected.start().await,
            LifecycleCommand::Restart => selected.restart().await,
            LifecycleCommand::Stop => {
                let output = selected.stop().await?;
                if let Err(err) = thread_recovery::discard_pending(&selected) {
                    eprintln!("warning: failed to clear saved threads after daemon stop: {err}");
                }
                Ok(output)
            }
            LifecycleCommand::Version => unreachable!(),
        }
    }

    async fn start(&self) -> Result<LifecycleOutput> {
        let mut managed = self.clone();
        let settings = self.load_settings().await?;
        let (status, backend, pid, info) = if let Ok(info) = client::probe(&self.socket_path).await
        {
            (
                LifecycleStatus::AlreadyRunning,
                self.running_backend(&settings).await?,
                None,
                info,
            )
        } else if self.running_backend_instance(&settings).await?.is_some() {
            (
                LifecycleStatus::AlreadyRunning,
                Some(BackendKind::Pid),
                None,
                self.wait_until_ready().await?,
            )
        } else {
            // A fresh start must ignore snapshots left by older stop clients.
            if let Err(err) = thread_recovery::discard_pending(self) {
                eprintln!("warning: failed to clear stale daemon recovery before start: {err}");
            }
            if self.host_command_args.is_none() {
                prepare_install::prepare(self, &settings, None).await?;
                managed.managed_ruxex_bin = self.current_managed_ruxex_bin()?;
            }
            managed.ensure_managed_ruxex_bin()?;
            let pid = managed.start_managed_backend(&settings).await?;
            (
                LifecycleStatus::Started,
                Some(BackendKind::Pid),
                pid,
                self.wait_until_ready().await?,
            )
        };
        Ok(managed
            .output(status, backend, pid, Some(info.app_server_version))
            .await)
    }

    async fn restart(&self) -> Result<LifecycleOutput> {
        let settings = self.load_settings().await?;
        if client::probe(&self.socket_path).await.is_ok()
            && self.running_backend(&settings).await?.is_none()
        {
            return Err(anyhow!(
                "app server is running but is not managed by ruxex app-server daemon"
            ));
        }
        if self.host_command_args.is_none() {
            prepare_install::prepare(self, &settings, None).await?;
        }
        let mut managed = self.clone();
        if self.host_command_args.is_none() {
            managed.managed_ruxex_bin = self.current_managed_ruxex_bin()?;
        }

        managed.ensure_managed_ruxex_bin()?;
        if let Some(backend) = self.running_backend_instance(&settings).await? {
            if let Err(err) = thread_recovery::discard_pending(self) {
                eprintln!("warning: failed to clear stale daemon recovery before restart: {err}");
            }
            backend
                .stop_with_grace(settings.shutdown_grace_seconds)
                .await?;
        }

        let pid = managed.start_managed_backend(&settings).await?;
        let info = self.wait_until_ready().await?;
        Ok(managed
            .output(
                LifecycleStatus::Restarted,
                Some(BackendKind::Pid),
                pid,
                Some(info.app_server_version),
            )
            .await)
    }

    async fn stop(&self) -> Result<LifecycleOutput> {
        let settings = DaemonSettings::load_for_stop(&self.settings_file).await;
        if let Some(backend) = self.running_backend_instance(&settings).await? {
            backend
                .stop_with_grace(settings.shutdown_grace_seconds)
                .await?;
            return Ok(self
                .output(
                    LifecycleStatus::Stopped,
                    Some(BackendKind::Pid),
                    /*pid*/ None,
                    /*app_server_version*/ None,
                )
                .await);
        }

        if client::probe(&self.socket_path).await.is_ok() {
            return Err(anyhow!(
                "app server is running but is not managed by ruxex app-server daemon"
            ));
        }

        Ok(self
            .output(
                LifecycleStatus::NotRunning,
                /*backend*/ None,
                /*pid*/ None,
                /*app_server_version*/ None,
            )
            .await)
    }

    async fn version(&self) -> Result<LifecycleOutput> {
        let settings = self.load_settings().await?;
        let info = client::probe(&self.socket_path).await?;
        Ok(self
            .output(
                LifecycleStatus::Running,
                self.running_backend(&settings).await?,
                /*pid*/ None,
                Some(info.app_server_version),
            )
            .await)
    }

    async fn wait_until_ready(&self) -> Result<client::ProbeInfo> {
        let deadline = tokio::time::Instant::now() + START_TIMEOUT;
        loop {
            match client::probe(&self.socket_path).await {
                Ok(info) => return Ok(info),
                Err(err) if tokio::time::Instant::now() < deadline => {
                    let _ = err;
                    sleep(START_POLL_INTERVAL).await;
                }
                Err(err) => {
                    let context = self.app_server_not_ready_context().await;
                    return Err(err).context(context);
                }
            }
        }
    }

    async fn app_server_not_ready_context(&self) -> String {
        let mut context = format!(
            "app server did not become ready on {}",
            self.socket_path.display()
        );
        self.append_daemon_app_server_context(&mut context).await;
        backend::append_stderr_log_tail_context(&self.pid_file, &mut context).await;
        context
    }

    async fn append_daemon_app_server_context(&self, context: &mut String) {
        let managed_ruxex_version = self
            .managed_ruxex_version_best_effort()
            .await
            .unwrap_or_else(|| "unknown".to_string());
        context.push_str(&format!(
            "\n\nDaemon used app-server:\n  path: {}\n  version: {managed_ruxex_version}",
            self.managed_ruxex_bin.display()
        ));
    }

    async fn bootstrap(
        &self,
        options: BootstrapOptions,
        package_source: Option<&Path>,
    ) -> Result<BootstrapOutput> {
        let _operation_lock = self.acquire_operation_lock().await?;
        self.current_installation()?
            .bootstrap_locked(options, package_source)
            .await
    }

    async fn bootstrap_with_host_args(
        &self,
        options: BootstrapOptions,
        package_source: &Path,
        host_command_args: &[OsString],
    ) -> Result<BootstrapOutput> {
        let _operation_lock = self.acquire_operation_lock().await?;
        self.current_installation()?
            .bootstrap_locked_with_host_args(options, Some(package_source), host_command_args)
            .await
    }

    async fn bootstrap_locked(
        &self,
        _options: BootstrapOptions,
        package_source: Option<&Path>,
    ) -> Result<BootstrapOutput> {
        self.bootstrap_locked_with_host_args(_options, package_source, &[])
            .await
    }

    async fn bootstrap_locked_with_host_args(
        &self,
        _options: BootstrapOptions,
        package_source: Option<&Path>,
        host_command_args: &[OsString],
    ) -> Result<BootstrapOutput> {
        let settings = self.load_settings().await?;
        if client::probe(&self.socket_path).await.is_ok()
            && self.running_backend(&settings).await?.is_none()
        {
            return Err(anyhow!(
                "app server is running but is not managed by ruxex app-server daemon"
            ));
        }
        prepare_install::prepare(self, &settings, package_source).await?;
        let mut managed = self.clone();
        managed.managed_ruxex_bin = self.current_managed_ruxex_bin()?;
        if !host_command_args.is_empty() {
            managed.host_command_args = Some(host_command_args.to_vec());
        }
        managed.ensure_managed_ruxex_bin()?;
        settings.save(&self.settings_file).await?;

        if let Some(backend) = self.running_backend_instance(&settings).await? {
            if let Err(err) = thread_recovery::discard_pending(self) {
                eprintln!("warning: failed to clear stale daemon recovery before bootstrap: {err}");
            }
            backend
                .stop_with_grace(settings.shutdown_grace_seconds)
                .await?;
        }

        let backend = backend::pid_backend(managed.backend_paths(&settings));
        backend.start().await?;
        let info = self.wait_until_ready().await?;
        let managed_ruxex_version = managed.managed_ruxex_version_best_effort().await;
        Ok(BootstrapOutput {
            status: BootstrapStatus::Bootstrapped,
            backend: BackendKind::Pid,
            managed_ruxex_path: managed.managed_ruxex_bin,
            managed_ruxex_version,
            socket_path: self.socket_path.clone(),
            cli_version: env!("CARGO_PKG_VERSION").to_string(),
            app_server_version: info.app_server_version,
        })
    }

    async fn running_backend(&self, settings: &DaemonSettings) -> Result<Option<BackendKind>> {
        Ok(self
            .running_backend_instance(settings)
            .await?
            .map(|_| BackendKind::Pid))
    }

    async fn running_backend_instance(
        &self,
        settings: &DaemonSettings,
    ) -> Result<Option<backend::PidBackend>> {
        let backend = backend::pid_backend(self.backend_paths(settings));
        if backend.is_starting_or_running().await? {
            return Ok(Some(backend));
        }
        Ok(None)
    }

    async fn start_managed_backend(&self, settings: &DaemonSettings) -> Result<Option<u32>> {
        self.start_managed_backend_with_bin(settings, &self.managed_ruxex_bin)
            .await
    }

    async fn start_managed_backend_with_bin(
        &self,
        settings: &DaemonSettings,
        managed_ruxex_bin: &Path,
    ) -> Result<Option<u32>> {
        let backend =
            backend::pid_backend(self.backend_paths_with_bin(settings, managed_ruxex_bin));
        backend.start().await
    }

    fn current_managed_ruxex_bin(&self) -> Result<PathBuf> {
        if self.host_command_args.is_some() {
            return Ok(self.managed_ruxex_bin.clone());
        }
        let home = self
            .settings_file
            .parent()
            .and_then(Path::parent)
            .context("daemon settings path has no Ruxex home")?;
        Ok(managed_install::managed_ruxex_bin(home))
    }

    fn ensure_managed_ruxex_bin(&self) -> Result<()> {
        if self.managed_ruxex_bin.is_file() {
            return Ok(());
        }

        let managed_ruxex_path = self.managed_ruxex_bin.display();
        Err(anyhow!(
            "daemon executable not found at {managed_ruxex_path}; repair the existing installation, or run `ruxex app-server daemon start` to install a missing daemon"
        ))
    }

    #[cfg(unix)]
    async fn managed_ruxex_version_best_effort(&self) -> Option<String> {
        if self.host_command_args.is_some() {
            return None;
        }
        managed_ruxex_version(&self.managed_ruxex_bin).await.ok()
    }

    fn backend_paths(&self, _settings: &DaemonSettings) -> BackendPaths {
        self.backend_paths_with_bin(_settings, &self.managed_ruxex_bin)
    }

    fn backend_paths_with_bin(
        &self,
        _settings: &DaemonSettings,
        managed_ruxex_bin: &Path,
    ) -> BackendPaths {
        BackendPaths {
            ruxex_bin: managed_ruxex_bin.to_path_buf(),
            pid_file: self.pid_file.clone(),
            command_args: self.host_command_args.clone(),
        }
    }

    async fn load_settings(&self) -> Result<DaemonSettings> {
        DaemonSettings::load(&self.settings_file).await
    }

    async fn acquire_operation_lock(&self) -> Result<tokio::fs::File> {
        let operation_lock = self.open_operation_lock_file().await?;
        let deadline = tokio::time::Instant::now() + OPERATION_LOCK_TIMEOUT;
        while !try_lock_file(&operation_lock)? {
            if tokio::time::Instant::now() >= deadline {
                return Err(anyhow!(
                    "timed out waiting for daemon operation lock {}",
                    self.operation_lock_file.display()
                ));
            }
            sleep(START_POLL_INTERVAL).await;
        }
        Ok(operation_lock)
    }

    async fn open_operation_lock_file(&self) -> Result<tokio::fs::File> {
        if let Some(parent) = self.operation_lock_file.parent() {
            #[cfg(unix)]
            if let Some(home) = parent.parent() {
                tokio::fs::create_dir_all(home).await?;
            }
            ruxex_uds::prepare_private_socket_directory(parent)
                .await
                .with_context(|| {
                    format!(
                        "failed to create daemon state directory {}",
                        parent.display()
                    )
                })?;
        }
        tokio::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&self.operation_lock_file)
            .await
            .with_context(|| {
                format!(
                    "failed to open daemon operation lock {}",
                    self.operation_lock_file.display()
                )
            })
    }

    async fn output(
        &self,
        status: LifecycleStatus,
        backend: Option<BackendKind>,
        pid: Option<u32>,
        app_server_version: Option<String>,
    ) -> LifecycleOutput {
        let managed_ruxex_version = self.managed_ruxex_version_best_effort().await;
        LifecycleOutput {
            status,
            backend,
            pid,
            managed_ruxex_path: self.managed_ruxex_bin.clone(),
            managed_ruxex_version,
            socket_path: self.socket_path.clone(),
            cli_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            app_server_version,
        }
    }
}

#[cfg(unix)]
fn try_lock_file(file: &tokio::fs::File) -> Result<bool> {
    use std::os::fd::AsRawFd;

    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        return Ok(true);
    }

    let err = std::io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::EWOULDBLOCK) {
        return Ok(false);
    }
    Err(err).context("failed to lock daemon operation")
}

#[cfg(all(test, unix))]
mod tests {
    use tempfile::TempDir;

    use super::Daemon;
    use super::LifecycleStatus;

    #[tokio::test]
    async fn waiting_lifecycle_command_uses_migrated_installation() {
        let home = TempDir::new().expect("home");
        let state = home.path().join("app-server-daemon");
        let legacy = home.path().join("packages/standalone/current");
        std::fs::create_dir_all(&legacy).expect("legacy selection");
        let daemon = Daemon {
            socket_path: home.path().join("server.sock"),
            pid_file: state.join(super::LEGACY_PID_FILE_NAME),
            operation_lock_file: state.join("daemon.lock"),
            settings_file: state.join("settings.json"),
            managed_ruxex_bin: super::managed_ruxex_bin(home.path()),
            host_command_args: None,
        };
        let lock = daemon.acquire_operation_lock().await.expect("lock");
        let stop = daemon.run(super::LifecycleCommand::Stop);
        tokio::pin!(stop);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), &mut stop)
                .await
                .is_err()
        );
        std::fs::create_dir_all(home.path().join("packages/app-server-daemon/current"))
            .expect("migrate selection");
        drop(lock);
        let output = stop.await.expect("stop");
        assert_eq!(
            (output.status, output.managed_ruxex_path),
            (
                LifecycleStatus::NotRunning,
                super::managed_ruxex_bin(home.path())
            )
        );
    }

    #[tokio::test]
    async fn stop_creates_missing_home_parent() {
        let temp = TempDir::new().expect("temp dir");
        let state = temp.path().join("missing-home").join("daemon-state");
        let daemon = Daemon {
            socket_path: state.join("server.sock"),
            pid_file: state.join("server.pid"),
            operation_lock_file: state.join("daemon.lock"),
            settings_file: state.join("settings.json"),
            managed_ruxex_bin: state.join("missing-ruxex"),
            host_command_args: None,
        };
        assert_eq!(
            daemon
                .run(super::LifecycleCommand::Stop)
                .await
                .expect("stop on fresh home")
                .status,
            LifecycleStatus::NotRunning,
        );
    }

    #[tokio::test]
    async fn stop_and_fresh_start_discard_pending_thread_restore() {
        let home = TempDir::new().expect("home");
        let state = home.path().join("app-server-daemon");
        ruxex_uds::prepare_private_socket_directory(&state)
            .await
            .expect("private state directory");
        let daemon = Daemon {
            socket_path: home.path().join("server.sock"),
            pid_file: state.join("server.pid"),
            operation_lock_file: state.join("daemon.lock"),
            settings_file: state.join("settings.json"),
            managed_ruxex_bin: home.path().join("ruxex"),
            host_command_args: None,
        };
        ruxex_app_server_transport::daemon_recovery::write_candidates(
            &daemon.recovery_file().expect("recovery path"),
            &["thread".to_string()].into_iter().collect(),
        )
        .expect("saved threads");
        assert_eq!(
            daemon
                .run(super::LifecycleCommand::Stop)
                .await
                .expect("stop")
                .status,
            LifecycleStatus::NotRunning
        );
        assert!(!daemon.recovery_file().expect("recovery path").exists());
        // Simulate an older stop client leaving a snapshot behind.
        std::fs::write(
            daemon.recovery_file().expect("recovery path"),
            r#"["thread"]"#,
        )
        .expect("legacy saved threads");
        daemon
            .run(super::LifecycleCommand::Start)
            .await
            .expect_err("missing backend binary");
        assert!(!daemon.recovery_file().expect("recovery path").exists());
    }

    #[tokio::test]
    async fn not_ready_context_reports_daemon_app_server_before_stderr() {
        let temp_dir = TempDir::new().expect("temp dir");
        let daemon = Daemon {
            socket_path: temp_dir.path().join("app-server-control.sock"),
            pid_file: temp_dir.path().join("app-server.pid"),
            operation_lock_file: temp_dir.path().join("daemon.lock"),
            settings_file: temp_dir.path().join("settings.json"),
            managed_ruxex_bin: temp_dir.path().join("missing-ruxex"),
            host_command_args: None,
        };
        let stderr_log = daemon.pid_file.with_extension("stderr.log");
        tokio::fs::write(&stderr_log, "unexpected argument")
            .await
            .expect("write stderr log");

        assert_eq!(
            daemon.app_server_not_ready_context().await,
            format!(
                "app server did not become ready on {}\n\n\
                 Daemon used app-server:\n  path: {}\n  version: unknown\n\n\
                 Managed app-server stderr ({}):\n  unexpected argument",
                daemon.socket_path.display(),
                daemon.managed_ruxex_bin.display(),
                stderr_log.display()
            )
        );
    }
}
