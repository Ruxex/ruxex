//! Cargo entry point for the minimal exec-server integration-test fixture.
//!
//! This mirrors the minimal exec-server fixture so Cargo-backed
//! app-server integration tests can receive `CARGO_BIN_EXE_exec-server`. It
//! also handles the helper argv modes because exec-server re-execs
//! `ruxex_self_exe` for sandboxed filesystem and process requests.

use ruxex_exec_server::ExecServerRuntimePaths;
use ruxex_http_client::HttpClientFactory;
use ruxex_http_client::OutboundProxyPolicy;
use std::ffi::OsStr;

const RUXEX_LINUX_SANDBOX_EXE_ENV_VAR: &str = "RUXEX_TEST_LINUX_SANDBOX_EXE";

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = std::env::args_os();
    let _ = args.next();
    let argv1 = args.next();
    #[cfg(unix)]
    if argv1.as_deref() == Some(OsStr::new(ruxex_exec_server::RUXEX_ARG0_EXEC_HELPER_ARG1)) {
        ruxex_exec_server::run_arg0_exec_helper_main();
    }
    if argv1.as_deref() == Some(OsStr::new(ruxex_exec_server::RUXEX_FS_HELPER_ARG1)) {
        ruxex_exec_server::run_fs_helper_main();
    }

    let current_exe = std::env::current_exe()?;
    let ruxex_linux_sandbox_exe =
        std::env::var_os(RUXEX_LINUX_SANDBOX_EXE_ENV_VAR).map(std::path::PathBuf::from);
    let runtime_paths = ExecServerRuntimePaths::new(current_exe, ruxex_linux_sandbox_exe)?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(ruxex_exec_server::run_main(
            "stdio",
            runtime_paths,
            // This test-only fixture has no application configuration to resolve HTTP policy.
            HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        ))
}
