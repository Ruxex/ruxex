//! Resolves an executor sandbox context to a concrete local sandbox implementation.

use ruxex_protocol::models::PermissionProfile;
use ruxex_sandboxing::SandboxManager;
use ruxex_sandboxing::SandboxType;
use ruxex_sandboxing::SandboxablePreference;

pub(crate) fn select_sandbox(
    manager: &SandboxManager,
    permission_profile: &PermissionProfile,
    has_managed_network_requirements: bool,
) -> SandboxType {
    manager.select_initial(
        permission_profile,
        SandboxablePreference::Require,
        has_managed_network_requirements,
    )
}
