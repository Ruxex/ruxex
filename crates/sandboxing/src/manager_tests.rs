use super::*;
use genex_protocol::models::PermissionProfile;

#[test]
fn unrestricted_forbidden_request_stays_unsandboxed() {
    let manager = SandboxManager::new();
    assert_eq!(
        manager.select_initial(
            &PermissionProfile::Disabled,
            SandboxablePreference::Forbid,
            false,
        ),
        SandboxType::None
    );
}

#[test]
fn required_request_uses_platform_sandbox_when_available() {
    let manager = SandboxManager::new();
    assert_eq!(
        manager.select_initial(
            &PermissionProfile::Disabled,
            SandboxablePreference::Require,
            false,
        ),
        get_platform_sandbox().unwrap_or(SandboxType::None)
    );
}
