use std::io::ErrorKind;
use std::path::Path;

use crate::rollout::SESSIONS_SUBDIR;
use genex_protocol::error::GenexErr;
use genex_thread_store::ThreadStoreError;

pub(crate) fn map_session_init_error(err: &anyhow::Error, genex_home: &Path) -> GenexErr {
    if let Some(store_error) = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<ThreadStoreError>())
    {
        match store_error {
            ThreadStoreError::Unsupported { operation } => {
                return GenexErr::UnsupportedOperation(format!("{operation} is not supported yet"));
            }
            ThreadStoreError::Conflict { message } => {
                return GenexErr::InvalidRequest(message.clone());
            }
            ThreadStoreError::ThreadNotFound { .. }
            | ThreadStoreError::InvalidRequest { .. }
            | ThreadStoreError::Internal { .. } => {}
        }
    }

    if let Some(mapped) = err
        .chain()
        .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
        .find_map(|io_err| map_rollout_io_error(io_err, genex_home))
    {
        return mapped;
    }

    GenexErr::Fatal(format!("Failed to initialize session: {err:#}"))
}

fn map_rollout_io_error(io_err: &std::io::Error, genex_home: &Path) -> Option<GenexErr> {
    let sessions_dir = genex_home.join(SESSIONS_SUBDIR);
    let hint = match io_err.kind() {
        ErrorKind::PermissionDenied => format!(
            "Codex cannot access session files at {} (permission denied). If sessions were created using sudo, fix ownership: sudo chown -R $(whoami) {}",
            sessions_dir.display(),
            genex_home.display()
        ),
        ErrorKind::NotFound => format!(
            "Session storage missing at {}. Create the directory or choose a different Genex home.",
            sessions_dir.display()
        ),
        ErrorKind::AlreadyExists => format!(
            "Session storage path {} is blocked by an existing file. Remove or rename it so Codex can create sessions.",
            sessions_dir.display()
        ),
        ErrorKind::InvalidData => format!(
            "Session data under {} looks corrupt or unreadable. Clearing the sessions directory may help (this will remove saved threads).",
            sessions_dir.display()
        ),
        ErrorKind::IsADirectory | ErrorKind::NotADirectory => format!(
            "Session storage path {} has an unexpected type. Ensure it is a directory Codex can use for session files.",
            sessions_dir.display()
        ),
        _ => return None,
    };

    Some(GenexErr::Fatal(format!(
        "{hint} (underlying error: {io_err})"
    )))
}
