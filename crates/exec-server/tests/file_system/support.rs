use std::fmt;
use std::sync::Arc;

use anyhow::Result;
use ruxex_exec_server::ExecServerRuntimePaths;
use ruxex_exec_server::ExecutorFileSystem;
use ruxex_exec_server::FileSystemSandboxContext;
use ruxex_exec_server::LocalFileSystem;
use ruxex_protocol::models::PermissionProfile;
use ruxex_protocol::permissions::FileSystemAccessMode;
use ruxex_protocol::permissions::FileSystemPath;
use ruxex_protocol::permissions::FileSystemSandboxEntry;
use ruxex_protocol::permissions::FileSystemSandboxPolicy;
use ruxex_protocol::permissions::FileSystemSpecialPath;
use ruxex_protocol::permissions::NetworkSandboxPolicy;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_path_uri::PathUri;

use crate::common::exec_server::TestRuxexHelperPaths;
use crate::common::exec_server::test_ruxex_helper_paths;

pub(crate) struct FileSystemContext {
    pub(crate) file_system: Arc<dyn ExecutorFileSystem>,
    _helper_paths: Option<TestRuxexHelperPaths>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum FileSystemImplementation {
    Local,
}

impl fmt::Display for FileSystemImplementation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local => formatter.write_str("local"),
        }
    }
}

pub(crate) async fn create_file_system_context(
    implementation: FileSystemImplementation,
) -> Result<FileSystemContext> {
    match implementation {
        FileSystemImplementation::Local => {
            let helper_paths = test_ruxex_helper_paths()?;
            let runtime_paths = ExecServerRuntimePaths::new(
                helper_paths.ruxex_exe.clone(),
                helper_paths.ruxex_linux_sandbox_exe.clone(),
            )?;
            Ok(FileSystemContext {
                file_system: Arc::new(LocalFileSystem::with_runtime_paths(runtime_paths)),
                _helper_paths: Some(helper_paths),
            })
        }
    }
}

pub(crate) fn absolute_path(path: std::path::PathBuf) -> AbsolutePathBuf {
    assert!(
        path.is_absolute(),
        "path must be absolute: {}",
        path.display()
    );
    AbsolutePathBuf::try_from(path).expect("path should be absolute")
}

pub(crate) fn read_only_sandbox(readable_root: std::path::PathBuf) -> FileSystemSandboxContext {
    let readable_root = absolute_path(readable_root);
    let cwd = PathUri::from_abs_path(&readable_root);
    let entries = vec![FileSystemSandboxEntry {
        path: FileSystemPath::Path {
            path: readable_root.into(),
        },
        access: FileSystemAccessMode::Read,
        missing_path_behavior: None,
    }];
    sandbox_context(entries, cwd)
}

pub(crate) fn workspace_write_sandbox(
    writable_root: std::path::PathBuf,
) -> FileSystemSandboxContext {
    let writable_root = absolute_path(writable_root);
    let cwd = PathUri::from_abs_path(&writable_root);
    let entries = vec![FileSystemSandboxEntry {
        path: FileSystemPath::Path {
            path: writable_root.into(),
        },
        access: FileSystemAccessMode::Write,
        missing_path_behavior: None,
    }];
    sandbox_context(entries, cwd)
}

fn sandbox_context(entries: Vec<FileSystemSandboxEntry>, cwd: PathUri) -> FileSystemSandboxContext {
    FileSystemSandboxContext::from_permission_profile(
        PermissionProfile::from_runtime_permissions(
            &FileSystemSandboxPolicy::restricted(entries),
            NetworkSandboxPolicy::Restricted,
        ),
        cwd,
    )
}
