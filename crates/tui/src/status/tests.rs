use super::new_status_output;
use super::new_status_output_with_rate_limits;
use super::new_status_output_with_rate_limits_handle;
use super::rate_limit_snapshot_display;
use super::rate_limits::RateLimitSnapshotDisplay;
use super::rate_limits::RateLimitWindowDisplay;
use super::rate_limits::SpendControlLimitSnapshotDisplay;
use super::rate_limits::StatusRateLimitData;
use super::rate_limits::compose_rate_limit_data_many;
use crate::history_cell::HistoryCell;
use crate::history_cell::PlainHistoryCell;
use crate::keymap::RuntimeKeymap;
use crate::legacy_core::config::Config;
use crate::legacy_core::config::ConfigBuilder;
use crate::legacy_core::config::PermissionProfileSnapshot;
use crate::pager_overlay::TranscriptOverlay;
use crate::status::StatusAccountDisplay;
use crate::status::remote_connection::RemoteConnectionStatus;
use crate::test_support::PathBufExt;
use crate::test_support::test_path_buf;
use crate::token_usage::TokenUsage;
use crate::token_usage::TokenUsageInfo;
use app_test_support::ApiKeyAuthFixture;
use app_test_support::write_api_key_auth;
use app_test_support::write_models_cache;
use chrono::Duration as ChronoDuration;
use chrono::Local;
use chrono::TimeZone;
use chrono::Utc;
use genex_app_server_protocol::AskForApproval;
use genex_app_server_protocol::CreditsSnapshot;
use genex_app_server_protocol::RateLimitSnapshot;
use genex_app_server_protocol::RateLimitWindow;
use genex_app_server_protocol::SpendControlLimitSnapshot;
use genex_config::LoaderOverrides;
use genex_config::types::AuthCredentialsStoreMode;
use genex_model_provider_info::ModelProviderAwsAuthInfo;
use genex_model_provider_info::ModelProviderInfo;
use genex_models_manager::test_support::construct_model_info_offline_for_tests;
use genex_models_manager::test_support::get_model_offline_for_tests;
use genex_protocol::ThreadId;
use genex_protocol::config_types::ApprovalsReviewer;
use genex_protocol::config_types::ReasoningSummary;
use genex_protocol::models::ActivePermissionProfile;
use genex_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE;
use genex_protocol::models::ManagedFileSystemPermissions;
use genex_protocol::models::PermissionProfile;
use genex_protocol::model_types::ReasoningEffort;
use genex_protocol::permissions::FileSystemAccessMode;
use genex_protocol::permissions::FileSystemPath;
use genex_protocol::permissions::FileSystemSandboxEntry;
use genex_protocol::permissions::FileSystemSpecialPath;
use genex_protocol::permissions::NetworkSandboxPolicy;
use genex_utils_absolute_path::AbsolutePathBuf;
use genex_utils_path_uri::PathUri;
use pretty_assertions::assert_eq;
use ratatui::prelude::*;
use std::sync::Arc;
use tempfile::TempDir;
use unicode_width::UnicodeWidthStr;

#[test]
fn stale_monthly_limit_marks_fresh_rolling_snapshot_stale() {
    let now = Local::now();
    let snapshot = RateLimitSnapshotDisplay {
        limit_name: "codex".to_string(),
        normal_model_slug: None,
        captured_at: now,
        primary: Some(RateLimitWindowDisplay {
            used_percent: 20.0,
            resets_at: Some("soon".to_string()),
            window_minutes: Some(300),
        }),
        secondary: None,
        credits: None,
        individual_limit: Some(SpendControlLimitSnapshotDisplay {
            captured_at: now - ChronoDuration::minutes(20),
            percent_remaining: 68.0,
            used: "8,000".to_string(),
            limit: "25,000".to_string(),
            resets_at: Some("later".to_string()),
        }),
    };

    assert!(matches!(
        compose_rate_limit_data_many(&[snapshot], now),
        StatusRateLimitData::Stale(_)
    ));
}

fn app_server_workspace_write_profile(network_enabled: bool) -> PermissionProfile {
    PermissionProfile::Managed {
        network: if network_enabled {
            NetworkSandboxPolicy::Enabled
        } else {
            NetworkSandboxPolicy::Restricted
        },
        file_system: ManagedFileSystemPermissions::Restricted {
            entries: vec![
                FileSystemSandboxEntry {
                    path: FileSystemPath::Special {
                        value: FileSystemSpecialPath::Root,
                    },
                    access: FileSystemAccessMode::Read,
                    missing_path_behavior: None,
                },
                FileSystemSandboxEntry {
                    path: FileSystemPath::Special {
                        value: FileSystemSpecialPath::ProjectRoots { subpath: None },
                    },
                    access: FileSystemAccessMode::Write,
                    missing_path_behavior: None,
                },
                FileSystemSandboxEntry {
                    path: FileSystemPath::Special {
                        value: FileSystemSpecialPath::SlashTmp,
                    },
                    access: FileSystemAccessMode::Write,
                    missing_path_behavior: None,
                },
                FileSystemSandboxEntry {
                    path: FileSystemPath::Special {
                        value: FileSystemSpecialPath::Tmpdir,
                    },
                    access: FileSystemAccessMode::Write,
                    missing_path_behavior: None,
                },
            ],
            glob_scan_max_depth: None,
        },
    }
}

async fn test_config(temp_home: &TempDir) -> Config {
    let mut config = ConfigBuilder::default()
        .codex_home(temp_home.path().to_path_buf())
        .loader_overrides(LoaderOverrides::without_managed_config_for_tests())
        .build()
        .await
        .expect("load config");
    config.approvals_reviewer = ApprovalsReviewer::User;
    config
        .permissions
        .set_permission_profile(app_server_workspace_write_profile(
            /*network_enabled*/ true,
        ))
        .expect("set permission profile");
    config
}

fn set_workspace_cwd(config: &mut Config, cwd: AbsolutePathBuf) {
    config.cwd = cwd.clone();
    config.workspace_roots = vec![cwd];
    config
        .permissions
        .set_workspace_roots(config.workspace_roots.clone());
}

fn test_status_account_display() -> Option<StatusAccountDisplay> {
    None
}

fn token_info_for(model_slug: &str, config: &Config, usage: &TokenUsage) -> TokenUsageInfo {
    let context_window =
        construct_model_info_offline_for_tests(model_slug, &config.to_models_manager_config())
            .context_window;
    TokenUsageInfo {
        total_token_usage: usage.clone(),
        last_token_usage: usage.clone(),
        model_context_window: context_window,
    }
}

fn render_lines(lines: &[Line<'static>]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect()
}

fn sanitize_directory(lines: Vec<String>) -> Vec<String> {
    let frame_width = lines
        .iter()
        .find(|line| line.starts_with('╭'))
        .map(|line| UnicodeWidthStr::width(line.as_str()));
    lines
        .into_iter()
        .map(|line| {
            if let (Some(frame_width), Some(dir_pos), Some(pipe_idx)) =
                (frame_width, line.find("Directory: "), line.rfind('│'))
            {
                let prefix = &line[..dir_pos + "Directory: ".len()];
                let suffix = &line[pipe_idx..];
                let replacement = "[[workspace]]";
                let content_width = frame_width.saturating_sub(
                    UnicodeWidthStr::width(prefix) + UnicodeWidthStr::width(suffix),
                );
                let mut rebuilt = prefix.to_string();
                rebuilt.push_str(replacement);
                let replacement_width = UnicodeWidthStr::width(replacement);
                if content_width > replacement_width {
                    rebuilt.push_str(&" ".repeat(content_width - replacement_width));
                }
                rebuilt.push_str(suffix);
                rebuilt
            } else {
                line
            }
        })
        .collect()
}

fn buffer_to_text(buffer: &Buffer, width: u16) -> String {
    let lines = buffer
        .content
        .chunks(usize::from(width))
        .map(|row| {
            row.iter()
                .map(|cell| {
                    let symbol = cell.symbol();
                    symbol
                        .strip_prefix("\x1b]8;;")
                        .and_then(|symbol| symbol.split_once('\x07'))
                        .and_then(|(_, symbol)| symbol.strip_suffix("\x1b]8;;\x07"))
                        .unwrap_or(symbol)
                })
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>();
    sanitize_directory(lines).join("\n")
}

fn reset_at_from(captured_at: &chrono::DateTime<chrono::Local>, seconds: i64) -> i64 {
    (*captured_at + ChronoDuration::seconds(seconds))
        .with_timezone(&Utc)
        .timestamp()
}

fn permissions_text_for(config: &Config) -> Option<String> {
    permissions_text_for_width(config, /*width*/ 80)
}

fn permissions_text_for_width(config: &Config, width: u16) -> Option<String> {
    let usage = TokenUsage::default();
    let captured_at = chrono::Local
        .with_ymd_and_hms(2024, 1, 2, 3, 4, 5)
        .single()
        .expect("timestamp");
    let model_slug = get_model_offline_for_tests(config.model.as_deref());
    let composite = new_status_output(
        config,
        test_status_account_display().as_ref(),
        /*token_info*/ None,
        &usage,
        &None,
        /*thread_name*/ None,
        /*forked_from*/ None,
        /*rate_limits*/ None,
        None,
        captured_at,
        &model_slug,
        /*collaboration_mode*/ None,
        /*reasoning_effort_override*/ None,
    );
    render_lines(&composite.display_lines(width))
        .iter()
        .find(|line| line.contains("Permissions:"))
        .and_then(|line| {
            line.split("Permissions:")
                .nth(1)
                .map(str::trim)
                .map(|text| text.trim_end_matches('│'))
                .map(str::trim)
                .map(ToString::to_string)
        })
}


#[tokio::test]
async fn status_permissions_non_default_workspace_write_uses_workspace_label() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config.model = Some("gpt-5.1-codex-max".to_string());
    config.model_provider_id = "provider".to_string();
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())
        .expect("set approval policy");
    set_workspace_cwd(&mut config, test_path_buf("/workspace/tests").abs());
    config
        .permissions
        .set_permission_profile(app_server_workspace_write_profile(
            /*network_enabled*/ true,
        ))
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config).as_deref(),
        Some("Custom (workspace with network access, Ask for approval)")
    );
}

#[tokio::test]
async fn status_permissions_named_read_only_profile_shows_builtin_label() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())
        .expect("set approval policy");
    config
        .permissions
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::active(
            PermissionProfile::read_only(),
            ActivePermissionProfile::read_only(),
        ))
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config).as_deref(),
        Some("Read Only (Ask for approval)")
    );
}

#[tokio::test]
async fn status_permissions_read_only_profile_shows_additional_writable_roots() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())
        .expect("set approval policy");
    let extra_root = test_path_buf("/workspace/extra").abs();
    let file_system_policy = PermissionProfile::read_only()
        .file_system_sandbox_policy()
        .with_additional_writable_roots(config.cwd.as_path(), std::slice::from_ref(&extra_root));
    config
        .permissions
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::active(
            PermissionProfile::from_runtime_permissions(
                &file_system_policy,
                NetworkSandboxPolicy::Restricted,
            ),
            ActivePermissionProfile::read_only(),
        ))
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config).as_deref(),
        Some("Read Only (Ask for approval)")
    );
}

#[tokio::test]
async fn status_permissions_named_workspace_profile_shows_builtin_label() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())
        .expect("set approval policy");
    config
        .permissions
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::active(
            PermissionProfile::workspace_write(),
            ActivePermissionProfile::new(BUILT_IN_PERMISSION_PROFILE_WORKSPACE),
        ))
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config).as_deref(),
        Some("Workspace (Ask for approval)")
    );
}

#[tokio::test]
async fn status_permissions_workspace_auto_review_shows_reviewer_label() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config.approvals_reviewer = ApprovalsReviewer::AutoReview;
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())
        .expect("set approval policy");
    config
        .permissions
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::active(
            PermissionProfile::workspace_write(),
            ActivePermissionProfile::new(BUILT_IN_PERMISSION_PROFILE_WORKSPACE),
        ))
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config).as_deref(),
        Some("Workspace (Approve for me)")
    );
}

#[tokio::test]
async fn status_permissions_named_profile_shows_additional_writable_roots() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())
        .expect("set approval policy");
    let extra_root = test_path_buf("/workspace/extra").abs();
    config
        .permissions
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::active(
            PermissionProfile::workspace_write_with(
                std::slice::from_ref(&extra_root),
                NetworkSandboxPolicy::Restricted,
                /*exclude_tmpdir_env_var*/ false,
                /*exclude_slash_tmp*/ false,
            ),
            ActivePermissionProfile::new(BUILT_IN_PERMISSION_PROFILE_WORKSPACE),
        ))
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config).as_deref(),
        Some("Workspace (Ask for approval)")
    );
}

#[tokio::test]
async fn status_permissions_workspace_roots_show_additional_directories() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    set_workspace_cwd(&mut config, test_path_buf("/workspace/tests").abs());
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())
        .expect("set approval policy");
    let extra_root = test_path_buf("/workspace/extra").abs();
    config.workspace_roots = vec![config.cwd.clone(), extra_root.clone()];
    config
        .permissions
        .set_workspace_roots(config.workspace_roots.clone());
    config
        .permissions
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::active(
            PermissionProfile::workspace_write(),
            ActivePermissionProfile::new(":workspace"),
        ))
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config),
        Some(format!(
            "Workspace [{}] (Ask for approval)",
            extra_root.display()
        ))
    );
}

#[tokio::test]
async fn status_permissions_workspace_roots_include_profile_defined_directories() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    set_workspace_cwd(&mut config, test_path_buf("/workspace/tests").abs());
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())
        .expect("set approval policy");
    let profile_root = test_path_buf("/workspace/shared").abs();
    config
        .permissions
        .set_permission_profile_from_session_snapshot(
            PermissionProfileSnapshot::active_with_profile_workspace_roots(
                PermissionProfile::workspace_write_with(
                    std::slice::from_ref(&profile_root),
                    NetworkSandboxPolicy::Restricted,
                    /*exclude_tmpdir_env_var*/ false,
                    /*exclude_slash_tmp*/ false,
                ),
                ActivePermissionProfile::new(":workspace"),
                vec![profile_root.clone().into()],
            ),
        )
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config),
        Some(format!(
            "Workspace [{}] (Ask for approval)",
            profile_root.display()
        ))
    );
}

#[tokio::test]
async fn status_permissions_broadened_workspace_profile_shows_builtin_label() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())
        .expect("set approval policy");
    config
        .permissions
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::active(
            PermissionProfile::workspace_write_with(
                &[],
                NetworkSandboxPolicy::Enabled,
                /*exclude_tmpdir_env_var*/ false,
                /*exclude_slash_tmp*/ false,
            ),
            ActivePermissionProfile::new(BUILT_IN_PERMISSION_PROFILE_WORKSPACE),
        ))
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config).as_deref(),
        Some("Workspace with network access (Ask for approval)")
    );
}

#[tokio::test]
async fn status_permissions_user_defined_profile_shows_name() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config
        .permissions
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::active(
            PermissionProfile::read_only(),
            ActivePermissionProfile::new("locked"),
        ))
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config).as_deref(),
        Some("Profile locked (read-only, Ask for approval)")
    );
}




#[tokio::test]
async fn status_permissions_full_disk_managed_with_network_is_danger_full_access() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())
        .expect("set approval policy");
    config
        .permissions
        .set_permission_profile(PermissionProfile::Managed {
            network: NetworkSandboxPolicy::Enabled,
            file_system: ManagedFileSystemPermissions::Unrestricted,
        })
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config).as_deref(),
        Some("Custom (danger-full-access, Ask for approval)")
    );
}

#[tokio::test]
async fn status_permissions_full_disk_managed_without_network_is_external_sandbox() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())
        .expect("set approval policy");
    config
        .permissions
        .set_permission_profile(PermissionProfile::Managed {
            network: NetworkSandboxPolicy::Restricted,
            file_system: ManagedFileSystemPermissions::Unrestricted,
        })
        .expect("set permission profile");

    assert_eq!(
        permissions_text_for(&config).as_deref(),
        Some("Custom (external-sandbox, Ask for approval)")
    );
}





#[tokio::test]
async fn status_snapshot_shows_unlimited_credits() {
    let temp_home = TempDir::new().expect("temp home");
    let config = test_config(&temp_home).await;
    let account_display = test_status_account_display();
    let usage = TokenUsage::default();
    let captured_at = chrono::Local
        .with_ymd_and_hms(2024, 2, 3, 4, 5, 6)
        .single()
        .expect("timestamp");
    let snapshot = RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        normal_model_slug: None,
        primary: None,
        secondary: None,
        credits: Some(CreditsSnapshot {
            has_credits: true,
            unlimited: true,
            balance: None,
        }),
        individual_limit: None,
        spend_control_reached: None,
        plan_type: None,
        rate_limit_reached_type: None,
    };
    let rate_display = rate_limit_snapshot_display(&snapshot, captured_at);
    let model_slug = get_model_offline_for_tests(config.model.as_deref());
    let token_info = token_info_for(&model_slug, &config, &usage);
    let composite = new_status_output(
        &config,
        account_display.as_ref(),
        Some(&token_info),
        &usage,
        &None,
        /*thread_name*/ None,
        /*forked_from*/ None,
        Some(&rate_display),
        None,
        captured_at,
        &model_slug,
        /*collaboration_mode*/ None,
        /*reasoning_effort_override*/ None,
    );
    let rendered = render_lines(&composite.display_lines(/*width*/ 120));
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("Credits:") && line.contains("Unlimited")),
        "expected Credits: Unlimited line, got {rendered:?}"
    );
}

#[tokio::test]
async fn status_snapshot_shows_positive_credits() {
    let temp_home = TempDir::new().expect("temp home");
    let config = test_config(&temp_home).await;
    let account_display = test_status_account_display();
    let usage = TokenUsage::default();
    let captured_at = chrono::Local
        .with_ymd_and_hms(2024, 3, 4, 5, 6, 7)
        .single()
        .expect("timestamp");
    let snapshot = RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        normal_model_slug: None,
        primary: None,
        secondary: None,
        credits: Some(CreditsSnapshot {
            has_credits: true,
            unlimited: false,
            balance: Some("12.5".to_string()),
        }),
        individual_limit: None,
        spend_control_reached: None,
        plan_type: None,
        rate_limit_reached_type: None,
    };
    let rate_display = rate_limit_snapshot_display(&snapshot, captured_at);
    let model_slug = get_model_offline_for_tests(config.model.as_deref());
    let token_info = token_info_for(&model_slug, &config, &usage);
    let composite = new_status_output(
        &config,
        account_display.as_ref(),
        Some(&token_info),
        &usage,
        &None,
        /*thread_name*/ None,
        /*forked_from*/ None,
        Some(&rate_display),
        None,
        captured_at,
        &model_slug,
        /*collaboration_mode*/ None,
        /*reasoning_effort_override*/ None,
    );
    let rendered = render_lines(&composite.display_lines(/*width*/ 120));
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("Credits:") && line.contains("13 credits")),
        "expected Credits line with rounded credits, got {rendered:?}"
    );
}

#[tokio::test]
async fn status_snapshot_shows_available_credits_without_display_balance() {
    let temp_home = TempDir::new().expect("temp home");
    let config = test_config(&temp_home).await;
    let account_display = test_status_account_display();
    let usage = TokenUsage::default();
    let captured_at = chrono::Local
        .with_ymd_and_hms(2024, 4, 5, 6, 7, 8)
        .single()
        .expect("timestamp");
    let model_slug = get_model_offline_for_tests(config.model.as_deref());
    let token_info = token_info_for(&model_slug, &config, &usage);
    for balance in [
        None,
        Some(String::new()),
        Some("0".to_string()),
        Some("not-a-number".to_string()),
        Some("inf".to_string()),
    ] {
        let snapshot = RateLimitSnapshot {
            limit_id: None,
            limit_name: None,
            normal_model_slug: None,
            primary: None,
            secondary: None,
            credits: Some(CreditsSnapshot {
                has_credits: true,
                unlimited: false,
                balance,
            }),
            individual_limit: None,
            spend_control_reached: None,
            plan_type: None,
            rate_limit_reached_type: None,
        };
        let rate_display = rate_limit_snapshot_display(&snapshot, captured_at);
        let composite = new_status_output(
            &config,
            account_display.as_ref(),
            Some(&token_info),
            &usage,
            &None,
            /*thread_name*/ None,
            /*forked_from*/ None,
            Some(&rate_display),
            None,
            captured_at,
            &model_slug,
            /*collaboration_mode*/ None,
            /*reasoning_effort_override*/ None,
        );
        let rendered = render_lines(&composite.display_lines(/*width*/ 120));
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("Credits:") && line.contains("Available")),
            "expected Credits: Available line, got {rendered:?}"
        );
    }
}

#[tokio::test]
async fn status_snapshot_respects_unlimited_without_has_credits_flag() {
    let temp_home = TempDir::new().expect("temp home");
    let config = test_config(&temp_home).await;
    let account_display = test_status_account_display();
    let usage = TokenUsage::default();
    let captured_at = chrono::Local
        .with_ymd_and_hms(2024, 5, 6, 7, 8, 9)
        .single()
        .expect("timestamp");
    let snapshot = RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        normal_model_slug: None,
        primary: None,
        secondary: None,
        credits: Some(CreditsSnapshot {
            has_credits: false,
            unlimited: true,
            balance: None,
        }),
        individual_limit: None,
        spend_control_reached: None,
        plan_type: None,
        rate_limit_reached_type: None,
    };
    let rate_display = rate_limit_snapshot_display(&snapshot, captured_at);
    let model_slug = get_model_offline_for_tests(config.model.as_deref());
    let token_info = token_info_for(&model_slug, &config, &usage);
    let composite = new_status_output(
        &config,
        account_display.as_ref(),
        Some(&token_info),
        &usage,
        &None,
        /*thread_name*/ None,
        /*forked_from*/ None,
        Some(&rate_display),
        None,
        captured_at,
        &model_slug,
        /*collaboration_mode*/ None,
        /*reasoning_effort_override*/ None,
    );
    let rendered = render_lines(&composite.display_lines(/*width*/ 120));
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("Credits:") && line.contains("Unlimited")),
        "expected Credits: Unlimited line, got {rendered:?}"
    );
}

#[tokio::test]
async fn status_card_token_usage_excludes_cached_tokens() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config.model = Some("gpt-5.1-codex-max".to_string());
    set_workspace_cwd(&mut config, test_path_buf("/workspace/tests").abs());

    let account_display = test_status_account_display();
    let usage = TokenUsage {
        input_tokens: 1_200,
        cached_input_tokens: 200,
        output_tokens: 900,
        reasoning_output_tokens: 0,
        total_tokens: 2_100,
    };

    let now = chrono::Local
        .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
        .single()
        .expect("timestamp");

    let model_slug = get_model_offline_for_tests(config.model.as_deref());
    let token_info = token_info_for(&model_slug, &config, &usage);
    let composite = new_status_output(
        &config,
        account_display.as_ref(),
        Some(&token_info),
        &usage,
        &None,
        /*thread_name*/ None,
        /*forked_from*/ None,
        /*rate_limits*/ None,
        None,
        now,
        &model_slug,
        /*collaboration_mode*/ None,
        /*reasoning_effort_override*/ None,
    );
    let rendered = render_lines(&composite.display_lines(/*width*/ 120));

    assert!(
        rendered.iter().all(|line| !line.contains("cached")),
        "cached tokens should not be displayed, got: {rendered:?}"
    );
}












#[tokio::test]
async fn status_context_window_uses_last_usage() {
    let temp_home = TempDir::new().expect("temp home");
    let mut config = test_config(&temp_home).await;
    config.model_context_window = Some(272_000);

    let account_display = test_status_account_display();
    let total_usage = TokenUsage {
        input_tokens: 12_800,
        cached_input_tokens: 0,
        output_tokens: 879,
        reasoning_output_tokens: 0,
        total_tokens: 102_000,
    };
    let last_usage = TokenUsage {
        input_tokens: 12_800,
        cached_input_tokens: 0,
        output_tokens: 879,
        reasoning_output_tokens: 0,
        total_tokens: 13_679,
    };

    let now = chrono::Local
        .with_ymd_and_hms(2024, 6, 1, 12, 0, 0)
        .single()
        .expect("timestamp");

    let model_slug = get_model_offline_for_tests(config.model.as_deref());
    let token_info = TokenUsageInfo {
        total_token_usage: total_usage.clone(),
        last_token_usage: last_usage,
        model_context_window: config.model_context_window,
    };
    let composite = new_status_output(
        &config,
        account_display.as_ref(),
        Some(&token_info),
        &total_usage,
        &None,
        /*thread_name*/ None,
        /*forked_from*/ None,
        /*rate_limits*/ None,
        None,
        now,
        &model_slug,
        /*collaboration_mode*/ None,
        /*reasoning_effort_override*/ None,
    );
    let rendered_lines = render_lines(&composite.display_lines(/*width*/ 80));
    let context_line = rendered_lines
        .into_iter()
        .find(|line| line.contains("Context window"))
        .expect("context line");

    assert!(
        context_line.contains("13.7K used / 272K"),
        "expected context line to reflect last usage tokens, got: {context_line}"
    );
    assert!(
        !context_line.contains("102K"),
        "context line should not use total aggregated tokens, got: {context_line}"
    );
}
