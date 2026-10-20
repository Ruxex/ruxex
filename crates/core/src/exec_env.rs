pub use ruxex_apply_patch::RUXEX_APPLY_PATCH_PRESERVE_LINE_ENDINGS_ENV_VAR;
use ruxex_protocol::SessionId;
use ruxex_protocol::ThreadId;
#[cfg(test)]
use ruxex_protocol::config_types::EnvironmentVariablePattern;
use ruxex_protocol::config_types::ShellEnvironmentPolicy;
use ruxex_protocol::models::ActivePermissionProfile;
use ruxex_protocol::shell_environment;
use std::collections::HashMap;

pub use ruxex_protocol::shell_environment::RUXEX_SESSION_ID_ENV_VAR;
pub use ruxex_protocol::shell_environment::RUXEX_THREAD_ID_ENV_VAR;

pub(crate) const RUXEX_VERSION_ENV_VAR: &str = "RUXEX_VERSION";

/// Informational name of the active permission profile. Child processes can
/// overwrite this value, so it must not be treated as proof of enforcement.
pub const RUXEX_PERMISSION_PROFILE_ENV_VAR: &str = "RUXEX_PERMISSION_PROFILE";

/// Construct an environment map based on the rules in the specified policy. The
/// resulting map can be passed directly to `Command::envs()` after calling
/// `env_clear()` to ensure no unintended variables are leaked to the spawned
/// process.
///
/// The derivation follows the algorithm documented in the struct-level comment
/// for [`ShellEnvironmentPolicy`].
///
/// `RUXEX_THREAD_ID` is injected when a thread id is provided, even when
/// `include_only` is set.
pub fn create_env(
    policy: &ShellEnvironmentPolicy,
    thread_id: Option<ThreadId>,
) -> HashMap<String, String> {
    let thread_id = thread_id.map(|thread_id| thread_id.to_string());
    shell_environment::create_env(policy, thread_id.as_deref())
}

/// Exposes the shared root-session identity and harness version to shell commands.
pub(crate) fn inject_session_env(env: &mut HashMap<String, String>, session_id: SessionId) {
    env.insert(RUXEX_SESSION_ID_ENV_VAR.to_string(), session_id.to_string());
    env.remove(RUXEX_VERSION_ENV_VAR);
    env.insert(
        RUXEX_VERSION_ENV_VAR.to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
}

/// Injects the selected named permission profile into a shell tool's environment.
///
/// This is applied after the shell environment policy so the runtime-selected
/// profile wins over inherited or configured values.
pub(crate) fn inject_permission_profile_env(
    env: &mut HashMap<String, String>,
    active_permission_profile: Option<&ActivePermissionProfile>,
) {
    env.remove(RUXEX_PERMISSION_PROFILE_ENV_VAR);
    if let Some(active_permission_profile) = active_permission_profile {
        env.insert(
            RUXEX_PERMISSION_PROFILE_ENV_VAR.to_string(),
            active_permission_profile.id.clone(),
        );
    }
}

/// Enables line-ending preservation for older standalone patch executables,
/// including those provided by remote executors. Override inherited values so
/// they cannot restore the legacy normalization behavior.
pub fn inject_apply_patch_env(env: &mut HashMap<String, String>) {
    env.retain(|key, _| !key.eq_ignore_ascii_case(RUXEX_APPLY_PATCH_PRESERVE_LINE_ENDINGS_ENV_VAR));
    env.insert(
        RUXEX_APPLY_PATCH_PRESERVE_LINE_ENDINGS_ENV_VAR.to_string(),
        "1".to_string(),
    );
}

#[cfg(test)]
fn populate_env<I>(
    vars: I,
    policy: &ShellEnvironmentPolicy,
    thread_id: Option<ThreadId>,
) -> HashMap<String, String>
where
    I: IntoIterator<Item = (String, String)>,
{
    let thread_id = thread_id.map(|thread_id| thread_id.to_string());
    shell_environment::populate_env(vars, policy, thread_id.as_deref())
}

#[cfg(test)]
#[path = "exec_env_tests.rs"]
mod tests;
