use genex_arg0::Arg0DispatchPaths;
use genex_config::RequirementSource;
use genex_core::config::Config;
use genex_protocol::permissions::FileSystemAccessMode;
use genex_protocol::permissions::FileSystemPath;

use super::CheckStatus;
use super::DoctorCheck;
use super::push_path_detail;

pub(super) fn sandbox_check(config: &Config, arg0_paths: &Arg0DispatchPaths) -> DoctorCheck {
    let file_system_sandbox = config.permissions.file_system_sandbox_policy();
    let denied_entries = file_system_sandbox
        .entries
        .iter()
        .filter(|entry| entry.access == FileSystemAccessMode::Deny);
    let mut details = vec![
        format!(
            "approval policy: {:?}",
            config.permissions.approval_policy.value()
        ),
        format!("filesystem sandbox: {}", file_system_sandbox.kind),
        format!("denied-read rules: {}", denied_entries.clone().count()),
        format!(
            "denied-read glob rules: {}",
            denied_entries
                .filter(|entry| matches!(entry.path, FileSystemPath::GlobPattern { .. }))
                .count()
        ),
        format!(
            "glob scan max depth: {}",
            file_system_sandbox
                .glob_scan_max_depth
                .map_or_else(|| "unbounded".to_string(), |depth| depth.to_string())
        ),
        format!(
            "managed filesystem source: {}",
            match config
                .config_layer_stack
                .requirements()
                .filesystem
                .as_ref()
                .map(|value| &value.source)
            {
                None => "none",
                Some(RequirementSource::Unknown) => "unknown",
                Some(RequirementSource::EnterpriseManaged { .. }) => "cloud",
                Some(RequirementSource::Composite { .. }) => "multiple layers",
                Some(RequirementSource::MdmManagedPreferences { .. }) => "MDM",
                Some(RequirementSource::SystemRequirementsToml { .. }) => "system",
            }
        ),
        format!(
            "network sandbox: {}",
            config.permissions.network_sandbox_policy()
        ),
    ];

    let mut status = CheckStatus::Ok;
    let mut summary = "sandbox configuration is readable".to_string();
    if let Some(helper) = arg0_paths.genex_linux_sandbox_exe.as_deref()
        && !helper.exists()
    {
        status = CheckStatus::Warning;
        summary = "Linux sandbox helper path does not exist".to_string();
    }

    push_path_detail(
        &mut details,
        "Linux sandbox helper",
        arg0_paths.genex_linux_sandbox_exe.as_deref(),
    );
    push_path_detail(
        &mut details,
        "execve wrapper helper",
        arg0_paths.main_execve_wrapper_exe.as_deref(),
    );
    DoctorCheck::new("sandbox.helpers", "sandbox", status, summary).details(details)
}
