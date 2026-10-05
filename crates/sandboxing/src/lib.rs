#[cfg(target_os = "linux")]
mod bwrap;
mod denial;
pub mod landlock;
mod manager;
pub mod policy_transforms;
#[cfg(target_os = "macos")]
pub mod seatbelt;
mod spawn;
mod terminal_queries;
mod violation;

#[cfg(target_os = "linux")]
pub use bwrap::find_system_bwrap_in_path;
#[cfg(target_os = "linux")]
pub use bwrap::system_bwrap_warning;
pub use denial::is_likely_executor_managed_sandbox_denied;
pub use denial::is_likely_sandbox_denied;
pub use manager::SandboxCommand;
pub use manager::SandboxDirectSpawnTransformRequest;
pub use manager::SandboxExecRequest;
pub use manager::SandboxManager;
pub use manager::SandboxTransformError;
pub use manager::SandboxTransformRequest;
pub use manager::SandboxType;
pub use manager::SandboxablePreference;
pub use manager::compatibility_sandbox_policy_for_permission_profile;
pub use manager::get_platform_sandbox;
pub use manager::with_managed_mitm_ca_readable_root;
pub use spawn::SpawnRequest;
pub use spawn::spawn_process;
pub use violation::FileSystemSandboxViolation;
pub use violation::FileSystemSandboxViolationReason;
pub use violation::NetworkSandboxViolation;
pub use violation::SandboxViolationBackend;
pub use violation::SandboxViolationEvent;
pub use violation::record_filesystem_sandbox_violation;
pub use violation::record_network_sandbox_violation;
pub use violation::record_sandbox_violation;

use genex_protocol::error::GenexErr;

#[cfg(not(target_os = "linux"))]
pub fn system_bwrap_warning(
    _permission_profile: &genex_protocol::models::PermissionProfile,
) -> Option<String> {
    None
}

impl From<SandboxTransformError> for GenexErr {
    fn from(err: SandboxTransformError) -> Self {
        match err {
            error @ SandboxTransformError::InvalidCommandCwd { .. }
            | error @ SandboxTransformError::InvalidSandboxPolicyCwd { .. } => {
                GenexErr::InvalidRequest(error.to_string())
            }
            SandboxTransformError::MissingLinuxSandboxExecutable => {
                GenexErr::LandlockSandboxExecutableNotProvided
            }
            SandboxTransformError::EnvironmentNetworkProxy(message) => {
                GenexErr::UnsupportedOperation(message)
            }
            #[cfg(target_os = "macos")]
            SandboxTransformError::SeatbeltPreparation(message) => {
                GenexErr::UnsupportedOperation(message)
            }
            #[cfg(target_os = "linux")]
            SandboxTransformError::Wsl1UnsupportedForBubblewrap => {
                GenexErr::UnsupportedOperation(crate::bwrap::WSL1_BWRAP_WARNING.to_string())
            }
            #[cfg(not(target_os = "macos"))]
            SandboxTransformError::SeatbeltUnavailable => GenexErr::UnsupportedOperation(
                "seatbelt sandbox is only available on macOS".to_string(),
            ),
        }
    }
}
