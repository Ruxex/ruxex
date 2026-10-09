//! Detached process launch and PID publication.

use super::PidBackend;
use super::PidFileState;
use super::PidRecord;
use super::read_process_start_time;
use crate::managed_install::executable_identity;
use anyhow::Context;
use anyhow::Result;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

impl PidBackend {
    pub(super) async fn start_inner(&self) -> Result<Option<u32>> {
        if let Some(parent) = self.pid_file.parent() {
            genex_uds::prepare_private_socket_directory(parent)
                .await
                .with_context(|| format!("failed to create pid directory {}", parent.display()))?;
        }
        let reservation_lock = self.acquire_reservation_lock().await?;
        loop {
            match fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&self.pid_file)
                .await
            {
                Ok(pid_file) => {
                    drop(pid_file);
                    break;
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    match self.read_pid_file_state_with_lock_held().await? {
                        PidFileState::Missing => continue,
                        PidFileState::Running(record) => {
                            if self.record_is_active(&record).await? {
                                return Ok(None);
                            }
                            let _ = fs::remove_file(&self.pid_file).await;
                            continue;
                        }
                        PidFileState::Starting => {
                            unreachable!("lock holder cannot observe starting")
                        }
                    }
                }
                Err(err) => {
                    return Err(err).with_context(|| {
                        format!("failed to reserve pid file {}", self.pid_file.display())
                    });
                }
            }
        }
        // Pin the selected release across installer symlink/junction retargeting.
        let genex_bin = fs::canonicalize(&self.genex_bin)
            .await
            .unwrap_or_else(|_| self.genex_bin.clone());
        let launched_identity = executable_identity(&genex_bin).await.ok();
        let mut command = Command::new(&genex_bin);
        let stderr_log = match self.open_stderr_log().await {
            Ok(stderr_log) => stderr_log,
            Err(err) => {
                let _ = fs::remove_file(&self.pid_file).await;
                return Err(err);
            }
        };
        command
            .args(self.command_args())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(stderr_log.into_std().await));
        // Older or pinned managed binaries may predate this optional startup flag.
        if matches!(
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                Command::new(&genex_bin)
                    .args(["app-server", "--managed-daemon", "--help"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .kill_on_drop(true)
                    .status(),
            ).await,
            Ok(Ok(status)) if status.success()
        ) {
            command.arg("--managed-daemon");
        } else {
            let genex_home = self
                .pid_file
                .parent()
                .and_then(std::path::Path::parent)
                .context("daemon pid path has no Genex home")?;
            let recovery_file = genex_app_server_transport::daemon_recovery_file_path(genex_home);
            match fs::remove_file(&recovery_file).await {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    tracing::warn!(path = %recovery_file.display(), %err, "failed to clear daemon recovery state before legacy launch");
                }
            }
        }
        #[cfg(unix)]
        {
            unsafe {
                command.pre_exec(|| {
                    if libc::setsid() == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }

        let child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                let _ = fs::remove_file(&self.pid_file).await;
                return Err(err).with_context(|| {
                    format!(
                        "failed to spawn detached app-server process using {}",
                        self.genex_bin.display()
                    )
                });
            }
        };
        let pid = child
            .id()
            .context("spawned app-server process has no pid")?;
        let record = match async {
            let process_start_time = read_process_start_time(pid).await?;
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            let process_identity = super::identity::read_process_details(pid)
                .await
                .ok()
                .map(|(_, identity)| identity);
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            let process_identity = None;
            anyhow::Ok(PidRecord {
                pid,
                process_start_time,
                process_identity,
                executable_identity: launched_identity,
            })
        }
        .await
        {
            Ok(record) => record,
            Err(err) => {
                let _ = self.terminate_process(pid);
                let mut context =
                    format!("failed to record pid-managed app-server process {pid} startup");
                super::super::append_stderr_log_tail_context(&self.pid_file, &mut context).await;
                let _ = fs::remove_file(&self.pid_file).await;
                return Err(err).context(context);
            }
        };
        let contents = serde_json::to_vec(&record).context("failed to serialize pid record")?;
        let temp_pid_file = self.pid_file.with_extension("pid.tmp");
        if let Err(err) = fs::write(&temp_pid_file, &contents).await {
            let _ = self.terminate_process(pid);
            let _ = fs::remove_file(&self.pid_file).await;
            return Err(err).with_context(|| {
                format!("failed to write pid temp file {}", temp_pid_file.display())
            });
        }
        if let Err(err) = fs::rename(&temp_pid_file, &self.pid_file).await {
            let _ = self.terminate_process(pid);
            let _ = fs::remove_file(&temp_pid_file).await;
            let _ = fs::remove_file(&self.pid_file).await;
            return Err(err).with_context(|| {
                format!("failed to publish pid file {}", self.pid_file.display())
            });
        }
        drop(reservation_lock);
        Ok(Some(pid))
    }
}
