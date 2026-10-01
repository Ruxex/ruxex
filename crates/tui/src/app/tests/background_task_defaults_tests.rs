//! Request-level coverage for command-center session defaults and worktree creation.

use super::*;
use crate::app::agents_overview::AGENTS_OVERVIEW_VIEW_ID;
use crate::app::tests::session_lifecycle_requests::HistoryCapabilities;
use crate::app::tests::session_lifecycle_requests::recorded_params;
use crate::app::tests::session_lifecycle_requests::start_recording_app_server_with_history;
use crate::model_catalog::ModelCatalog;
use crate::test_support::PathBufExt;
use crate::tui::test_support::make_test_tui;
use genex_state::SqliteConfig;
use pretty_assertions::assert_eq;

async fn confirm_permission_selection(
    app: &mut App,
    server: &mut AppServerSession,
    thread_id: ThreadId,
) -> Result<()> {
    for _ in 0..20 {
        let settings = next_thread_settings_updated(server, thread_id).await;
        app.enqueue_thread_notification(
            thread_id,
            ServerNotification::ThreadSettingsUpdated(settings),
        )
        .await?;
        if !app.pending_server_profiles.contains_key(&thread_id) {
            return Ok(());
        }
    }
    color_eyre::eyre::bail!("permission update was not confirmed");
}

fn trust_launch_folder(app: &mut App) {
    let projects = serde_json::json!({
        app.config.cwd.display().to_string(): {"trust_level": "trusted"},
        app.chat_widget.config_ref().cwd.display().to_string(): {"trust_level": "trusted"},
    });
    app.cli_kv_overrides.push((
        "projects".into(),
        TomlValue::try_from(projects).expect("trust fixture"),
    ));
    app.config.active_project.trust_level = Some(genex_protocol::config_types::TrustLevel::Trusted);
}


#[tokio::test]
async fn command_center_new_reads_server_defaults_for_actual_destination() -> Result<()> {
    let mut tui = make_test_tui()?;
    tui.pause_events();
    for (mode, explicit_cwd, launch_override, expected_cwd, expected_model) in [
        ("local", false, false, "launch", "server-model"),
        ("local", true, false, "destination", "destination-model"),
        ("local-cli-provider", false, false, "launch", "server-model"),
        ("local-cli-model", false, false, "launch", "cli-model"),
        (
            "local-default-provider",
            false,
            false,
            "launch",
            "server-model",
        ),
        ("remote", true, true, "destination", "destination-model"),
        ("remote", true, false, "destination", "destination-model"),
        ("remote", false, true, "launch", "server-model"),
        ("remote", false, false, ".", "server-model"),
        ("remote-null-service-tier", false, false, ".", ""),
    ] {
        let client_home = tempdir()?;
        let server_home = tempdir()?;
        let launch = tempdir()?;
        let destination = tempdir()?;
        std::fs::write(
            client_home.path().join("config.toml"),
            format!(
                "model = \"{}\"\nmodel_reasoning_effort = \"low\"\n{}",
                if mode == "remote-null-service-tier" {
                    "gpt-5.2"
                } else {
                    "client-model"
                },
                if mode == "local-default-provider" {
                    "model_provider = \"ollama\"\n"
                } else {
                    ""
                }
            ),
        )?;
        std::fs::write(
            server_home.path().join("config.toml"),
            format!(
                "{}model_reasoning_effort = \"high\"\n{}",
                if mode == "remote-null-service-tier" {
                    ""
                } else {
                    "model = \"server-model\"\n"
                },
                if mode == "local" || mode == "local-cli-provider" || mode == "local-cli-model" {
                    "model_provider = \"ollama\"\n"
                } else {
                    ""
                }
            ),
        )?;
        std::fs::create_dir(destination.path().join(".codex"))?;
        std::fs::write(
            destination.path().join(".codex/config.toml"),
            "model = \"destination-model\"\nservice_tier = \"flex\"\n",
        )?;
        for home in [client_home.path(), server_home.path()] {
            crate::legacy_core::config::set_project_trust_level(
                home,
                destination.path(),
                genex_protocol::config_types::TrustLevel::Trusted,
            )
            .map_err(|error| color_eyre::eyre::eyre!(error.to_string()))?;
        }
        let mut app = make_test_app_with_channels().await.0;
        trust_launch_folder(&mut app);
        if mode == "local" && explicit_cwd {
            app.harness_overrides.model_provider = Some("provider".into());
        }
        if mode == "local-cli-provider" {
            app.cli_kv_overrides
                .push(("model_provider".into(), TomlValue::String("provider".into())));
        }
        if mode == "local-cli-model" {
            app.harness_overrides.model = Some("cli-model".into());
        }
        app.chat_widget.set_service_tier(Some("flex".into()));
        app.harness_overrides.cwd = Some(launch.path().to_path_buf());
        app.config = ConfigBuilder::default()
            .codex_home(client_home.path().to_path_buf())
            .loader_overrides(app.loader_overrides.clone())
            .cli_overrides(app.cli_kv_overrides.clone())
            .harness_overrides(app.harness_overrides.clone())
            .build()
            .await?;
        app.chat_widget
            .handle_thread_session_quiet(test_thread_session(
                ThreadId::new(),
                launch.path().to_path_buf(),
            ));
        trust_launch_folder(&mut app);
        let mut server_config = app.config.clone();
        server_config.codex_home = server_home.path().to_path_buf().abs();
        server_config.sqlite = SqliteConfig::new_for_testing(server_home.path().abs());
        let thread_mode = if mode.starts_with("remote") {
            crate::app_server_session::ThreadParamsMode::Remote
        } else {
            crate::app_server_session::ThreadParamsMode::Embedded
        };
        let (mut server, requests, proxy) = start_recording_app_server_with_history(
            &server_config,
            HistoryCapabilities::Current,
            /*blocked_thread_list*/ None,
            /*failed_thread_name*/ None,
            thread_mode,
            LoaderOverrides {
                user_config_path: Some(server_home.path().join("config.toml").abs()),
                ..LoaderOverrides::default()
            },
        )
        .await?;
        if launch_override {
            server = server.with_remote_cwd_override(Some(launch.path().to_path_buf()));
        }
        if mode.starts_with("remote") && !launch_override {
            app.app_server_target = AppServerTarget::Remote {
                endpoint: crate::RemoteAppServerEndpoint::WebSocket {
                    websocket_url: "ws://127.0.0.1:1".into(),
                    auth_token: None,
                },
            };
            let mut restored = app.config.clone();
            restored
                .permissions
                .set_permission_profile(genex_protocol::models::PermissionProfile::Disabled)?;
            app.runtime_permission_profile_override = Some(
                RuntimePermissionProfileOverride::from_restored_config(&restored),
            );
            app.runtime_approval_policy_override = Some(RuntimeApprovalPolicyOverride::Restored(
                genex_app_server_protocol::AskForApproval::Never,
            ));
        }
        let bootstrap = server.bootstrap(&app.config).await?;
        let expected_model = if mode == "remote-null-service-tier" {
            let default_model = bootstrap
                .available_models
                .iter()
                .find(|model| model.is_default)
                .or_else(|| bootstrap.available_models.first())
                .expect("server catalog model")
                .model
                .clone();
            app.model_catalog = Arc::new(ModelCatalog::new(bootstrap.available_models));
            default_model
        } else {
            expected_model.to_string()
        };
        app.new_agents_overview_session(
            &mut tui,
            &mut server,
            explicit_cwd.then(|| destination.path().to_path_buf().abs()),
        )
        .await?;
        let cwd = match expected_cwd {
            "launch" => launch.path().display().to_string(),
            "destination" => destination.path().display().to_string(),
            "." => ".".to_string(),
            _ => unreachable!(),
        };
        assert_eq!(
            recorded_params(&requests, "config/read"),
            vec![serde_json::json!({"cwd": cwd})],
            "{mode} {expected_cwd}"
        );
        let starts = recorded_params(&requests, "thread/start");
        assert_eq!(starts.len(), 1, "{mode} {expected_cwd}");
        assert_eq!(
            (
                &starts[0]["cwd"],
                &starts[0]["model"],
                &starts[0]["modelProvider"],
                &starts[0]["config"]["model_reasoning_effort"],
                &starts[0]["serviceTier"],
            ),
            (
                &if mode.starts_with("remote") && !explicit_cwd && !launch_override {
                    serde_json::Value::Null
                } else {
                    serde_json::json!(cwd)
                },
                &serde_json::json!(expected_model),
                &if mode.starts_with("remote") {
                    serde_json::Value::Null
                } else if mode == "local" && !explicit_cwd {
                    serde_json::json!("ollama")
                } else {
                    serde_json::json!("provider")
                },
                &serde_json::json!("high"),
                &serde_json::json!(if mode == "local" && explicit_cwd {
                    "flex"
                } else {
                    "flex"
                }),
            ),
            "{mode} {expected_cwd}"
        );
        if mode.starts_with("remote") && !launch_override {
            assert_eq!(app.config.cwd, launch.path().to_path_buf().abs());
            assert_ne!(starts[0]["sandbox"], "danger-full-access");
            assert_ne!(starts[0]["approvalPolicy"], "never");
        }
        assert_eq!(recorded_params(&requests, "turn/start").len(), 0);
        server.shutdown().await?;
        proxy.await??;
    }
    Ok(())
}

#[tokio::test]
async fn command_center_new_preserves_explicit_choices_and_managed_defaults() -> Result<()> {
    let mut tui = make_test_tui()?;
    tui.pause_events();
    for (choice, expected_model, expected_effort) in [
        ("saved", "server-model", "high"),
        ("cli_effort", "server-model", "low"),
        ("profile_model", "profile-model", "high"),
        ("profile_managed", "profile-model", "low"),
        ("managed", "managed-model", "medium"),
    ] {
        let client_home = tempdir()?;
        let server_home = tempdir()?;
        std::fs::write(
            client_home.path().join("config.toml"),
            "model = \"client-model\"\nmodel_reasoning_effort = \"low\"\n",
        )?;
        std::fs::write(
            server_home.path().join("config.toml"),
            "model = \"server-model\"\nmodel_reasoning_effort = \"high\"\n",
        )?;
        if choice == "managed" || choice == "profile_managed" || choice.starts_with("cli_") {
            std::fs::write(
                server_home.path().join("requirements.toml"),
                "[models.new_thread]\nmodel = \"managed-model\"\nmodel_reasoning_effort = \"medium\"\n",
            )?;
        }
        let mut app = make_test_app_with_channels().await.0;
        trust_launch_folder(&mut app);
        match choice {
            "cli_effort" => app.cli_kv_overrides.push((
                "model_reasoning_effort".into(),
                TomlValue::String("low".into()),
            )),
            "profile_model" | "profile_managed" => {
                let path = client_home.path().join("work.config.toml");
                std::fs::write(
                    &path,
                    if choice == "profile_managed" {
                        "model = \"profile-model\"\nmodel_reasoning_effort = \"low\"\n"
                    } else {
                        "model = \"profile-model\"\n"
                    },
                )?;
                app.loader_overrides.user_config_path = Some(path.abs());
                app.loader_overrides.user_config_profile = Some("work".parse()?);
            }
            _ => {}
        }
        app.config = ConfigBuilder::default()
            .codex_home(client_home.path().to_path_buf())
            .loader_overrides(app.loader_overrides.clone())
            .cli_overrides(app.cli_kv_overrides.clone())
            .harness_overrides(app.harness_overrides.clone())
            .build()
            .await?;
        trust_launch_folder(&mut app);
        let mut server_config = app.config.clone();
        server_config.codex_home = server_home.path().to_path_buf().abs();
        server_config.sqlite = SqliteConfig::new_for_testing(server_home.path().abs());
        let (mut server, requests, proxy) = start_recording_app_server_with_history(
            &server_config,
            HistoryCapabilities::Current,
            /*blocked_thread_list*/ None,
            /*failed_thread_name*/ None,
            crate::app_server_session::ThreadParamsMode::Remote,
            LoaderOverrides {
                user_config_path: Some(server_home.path().join("config.toml").abs()),
                system_requirements_path: Some(server_home.path().join("requirements.toml")),
                ..LoaderOverrides::default()
            },
        )
        .await?;
        server.bootstrap(&app.config).await?;
        app.new_agents_overview_session(&mut tui, &mut server, /*cwd*/ None)
            .await?;
        let starts = recorded_params(&requests, "thread/start");
        assert_eq!(starts.len(), 1, "{choice}");
        assert_eq!(
            (
                &starts[0]["model"],
                &starts[0]["config"]["model_reasoning_effort"]
            ),
            (
                &serde_json::json!(expected_model),
                &serde_json::json!(expected_effort)
            ),
            "{choice}"
        );
        assert_eq!(
            recorded_params(&requests, "turn/start").len(),
            0,
            "{choice}"
        );
        server.shutdown().await?;
        proxy.await??;
    }
    Ok(())
}


#[tokio::test]
async fn command_center_new_preserves_permissions_across_sessions() -> Result<()> {
    let mut app = make_test_app_with_channels().await.0;
    trust_launch_folder(&mut app);
    let (mut server, requests, proxy) = start_recording_app_server_with_history(
        &app.config,
        HistoryCapabilities::Current,
        /*blocked_thread_list*/ None,
        /*failed_thread_name*/ None,
        crate::app_server_session::ThreadParamsMode::Embedded,
        LoaderOverrides::default(),
    )
    .await?;
    assert!(
        app.apply_permission_profile_selection(PermissionProfileSelection {
            profile_id: ":read-only".into(),
            approval_policy: Some(AskForApproval::UnlessTrusted),
            approvals_reviewer: Some(ApprovalsReviewer::User),
            display_label: "Read Only".into(),
        })
        .await
    );
    let mut tui = make_test_tui()?;
    tui.pause_events();
    for _ in 0..2 {
        app.new_agents_overview_session(&mut tui, &mut server, /*cwd*/ None)
            .await?;
        assert_eq!(
            (
                app.chat_widget
                    .config_ref()
                    .permissions
                    .permission_profile(),
                app.chat_widget
                    .config_ref()
                    .permissions
                    .approval_policy
                    .value(),
                app.chat_widget
                    .config_ref()
                    .permissions
                    .active_permission_profile(),
            ),
            (
                &genex_protocol::models::PermissionProfile::read_only(),
                genex_protocol::protocol::AskForApproval::UnlessTrusted,
                Some(ActivePermissionProfile::new(":read-only")),
            ),
        );
    }
    assert_eq!(recorded_params(&requests, "thread/start").len(), 2);
    server.shutdown().await?;
    proxy.await??;
    Ok(())
}
