#![allow(clippy::expect_used)]

// These provider-independent v2 cases exercise the AppServer process and RPC
// surface directly. Provider-wire-format cases live in `regression.rs` and are
// included by the aggregate `just app-server-e2e` recipe.
#[rustfmt::skip]
#[path = "suite/v2/collaboration_mode_list.rs"]
mod collaboration_mode_list;
#[rustfmt::skip]
#[path = "suite/v2/connection_handling_websocket.rs"]
mod connection_handling_websocket;
#[cfg(unix)]
#[rustfmt::skip]
#[path = "suite/v2/command_exec.rs"]
mod command_exec;
#[rustfmt::skip]
#[path = "suite/v2/experimental_api.rs"]
mod experimental_api;
#[rustfmt::skip]
#[path = "suite/v2/fs.rs"]
mod fs;
#[cfg(unix)]
#[rustfmt::skip]
#[path = "suite/v2/process_exec.rs"]
mod process_exec;
#[rustfmt::skip]
#[path = "suite/v2/permission_profile_list.rs"]
mod permission_profile_list;
#[rustfmt::skip]
#[path = "suite/v2/request_validation.rs"]
mod request_validation;
#[rustfmt::skip]
#[path = "suite/v2/rollout_compress_tests.rs"]
mod rollout_compress;
#[rustfmt::skip]
#[path = "suite/v2/thread_loaded_list.rs"]
mod thread_loaded_list;
#[rustfmt::skip]
#[path = "suite/v2/thread_delete.rs"]
mod thread_delete;
#[rustfmt::skip]
#[path = "suite/v2/thread_sections.rs"]
mod thread_sections;
#[rustfmt::skip]
#[path = "suite/v2/user_verification.rs"]
mod user_verification;
