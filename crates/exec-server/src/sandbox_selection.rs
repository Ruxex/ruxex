//! Resolves an executor sandbox context to a concrete local sandbox implementation.

use genex_protocol::models::PermissionProfile;
use genex_sandboxing::SandboxManager;
use genex_sandboxing::SandboxType;
use genex_sandboxing::SandboxablePreference;

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
