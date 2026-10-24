#![allow(clippy::expect_used)]

// These provider-independent v2 cases exercise the AppServer process and RPC
// surface directly. Provider-wire-format cases remain in `regression.rs`.
#[rustfmt::skip]
#[path = "suite/v2/collaboration_mode_list.rs"]
mod collaboration_mode_list;
#[rustfmt::skip]
#[path = "suite/v2/experimental_api.rs"]
mod experimental_api;
#[rustfmt::skip]
#[path = "suite/v2/permission_profile_list.rs"]
mod permission_profile_list;
#[rustfmt::skip]
#[path = "suite/v2/request_validation.rs"]
mod request_validation;
