use std::ffi::c_void;
use std::io::Write;
use std::os::windows::fs::MetadataExt as _;
use std::path::PathBuf;

use crate::ensure_allow_mask_aces_with_inheritance;
use crate::path_mask_allows;
use anyhow::Result;
use windows_sys::Win32::Security::CONTAINER_INHERIT_ACE;
use windows_sys::Win32::Security::OBJECT_INHERIT_ACE;
use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
use windows_sys::Win32::Storage::FileSystem::FILE_GENERIC_EXECUTE;
use windows_sys::Win32::Storage::FileSystem::FILE_GENERIC_READ;

#[cfg(test)]
#[path = "setup_runtime_bin_tests.rs"]
mod tests;

pub(super) fn ensure_managed_runtime_paths_readable(
    sandbox_group_psid: *mut c_void,
    refresh_errors: &mut Vec<String>,
    log: &mut dyn Write,
) -> Result<()> {
    let read_execute_mask = FILE_GENERIC_READ | FILE_GENERIC_EXECUTE;
    let runtime_paths = runtime_paths(std::env::var_os("USERPROFILE").map(PathBuf::from));

    for runtime_path in runtime_paths {
        if !std::fs::symlink_metadata(&runtime_path).is_ok_and(|metadata| {
            metadata.is_dir() && (metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) == 0
        }) {
            continue;
        }

        let has_access = match path_mask_allows(
            &runtime_path,
            &[sandbox_group_psid],
            read_execute_mask,
            /*require_all_bits*/ true,
        ) {
            Ok(has_access) => has_access,
            Err(err) => {
                refresh_errors.push(format!(
                    "runtime read/execute mask check failed on {} for sandbox_group: {err}",
                    runtime_path.display()
                ));
                super::log_line(
                    log,
                    &format!(
                        "runtime read/execute mask check failed on {} for sandbox_group: {err}; continuing",
                        runtime_path.display()
                    ),
                )?;
                false
            }
        };
        if has_access {
            continue;
        }

        super::log_line(
            log,
            &format!(
                "granting read/execute ACE to {} for sandbox users",
                runtime_path.display()
            ),
        )?;
        let result = unsafe {
            ensure_allow_mask_aces_with_inheritance(
                &runtime_path,
                &[sandbox_group_psid],
                read_execute_mask,
                OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE,
            )
        };
        if let Err(err) = result {
            refresh_errors.push(format!(
                "grant read/execute ACE failed on {} for sandbox_group: {err}",
                runtime_path.display()
            ));
            super::log_line(
                log,
                &format!(
                    "grant read/execute ACE failed on {} for sandbox_group: {err}",
                    runtime_path.display()
                ),
            )?;
        }
    }
    Ok(())
}

fn runtime_paths(user_profile: Option<PathBuf>) -> Vec<PathBuf> {
    user_profile
        .into_iter()
        .map(|profile| profile.join(".cache").join("codex-runtimes"))
        .collect()
}
