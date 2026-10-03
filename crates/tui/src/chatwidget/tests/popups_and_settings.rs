use super::*;
use crate::app_event::ConnectorsSnapshot;
use crate::chatwidget::connectors::ConnectorsCacheState;
use genex_app_server_protocol::HookErrorInfo;
use genex_app_server_protocol::HooksListEntry;
use genex_app_server_protocol::HooksListResponse;
use genex_app_server_protocol::MarketplaceLoadErrorInfo;
use genex_app_server_protocol::MarketplaceRemoveResponse;
use genex_app_server_protocol::PluginAvailability;
use genex_app_server_protocol::PluginShareContext;
use genex_app_server_protocol::PluginShareDiscoverability;
use genex_app_server_protocol::PluginSource;
use codex_connectors::AppInfo;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn experimental_mode_plan_is_ignored_on_startup() {
    let genex_home = tempdir().expect("tempdir");
    let cfg = ConfigBuilder::default()
        .genex_home(genex_home.path().to_path_buf())
        .cli_overrides(vec![
            (
                "features.collaboration_modes".to_string(),
                TomlValue::Boolean(true),
            ),
            (
                "tui.experimental_mode".to_string(),
                TomlValue::String("plan".to_string()),
            ),
        ])
        .build()
        .await
        .expect("config");
    let resolved_model = get_model_offline_for_tests(cfg.model.as_deref());
    let session_telemetry = test_session_telemetry(&cfg, resolved_model.as_str());
    let init = ChatWidgetInit {
        requires_auth: true,
        local_settings: crate::local_settings::LocalSettings::from(&cfg),
        config: cfg.clone(),
        frame_requester: FrameRequester::test_dummy(),
        app_event_tx: AppEventSender::new(unbounded_channel::<AppEvent>().0),
        workspace_command_runner: None,
        initial_user_message: None,
        enhanced_keys_supported: false,
        has_managed_account: false,
        has_codex_backend_auth: false,
        model_catalog: test_model_catalog(&cfg),
        is_first_run: true,
        status_account_display: None,
        initial_plan_type: None,
        model: Some(resolved_model.clone()),
        startup_tooltip_override: None,
        status_line_invalid_items_warned: Arc::new(AtomicBool::new(false)),
        terminal_title_invalid_items_warned: Arc::new(AtomicBool::new(false)),
        session_telemetry,
    };

    let chat = ChatWidget::new_with_app_event(init);
    assert_eq!(chat.active_collaboration_mode_kind(), ModeKind::Default);
    assert_eq!(chat.current_model(), resolved_model);
}






#[tokio::test]
async fn plugins_popup_keeps_loaded_marketplace_state_with_load_errors() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let mut response = plugins_test_response(vec![plugins_test_curated_marketplace(Vec::new())]);
    response
        .marketplace_load_errors
        .push(MarketplaceLoadErrorInfo {
            marketplace_path: plugins_test_absolute_path("marketplaces/broken"),
            message: "failed to load marketplace manifest".to_string(),
        });

    let popup = render_loaded_plugins_popup(&mut chat, response);
    assert!(
        popup.contains("No marketplace plugins available")
            && !popup.contains("Marketplace unavailable"),
        "expected /plugins to keep the loaded marketplace state, got:\n{popup}"
    );
}


#[tokio::test]
async fn plugins_popup_add_marketplace_tab_opens_prompt_and_submits_source() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let cwd = chat.config.cwd.to_path_buf();
    render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![plugins_test_curated_marketplace(Vec::new())]),
    );

    while rx.try_recv().is_ok() {}
    let popup = select_plugins_tab_containing(
        &mut chat,
        /*width*/ 100,
        "Add a marketplace from a Git repo or local root.",
    );
    assert!(
        popup.contains("Add a marketplace from a Git repo or local root."),
        "expected Add marketplace tab, got:\n{popup}"
    );

    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    match rx.try_recv() {
        Ok(AppEvent::OpenMarketplaceAddPrompt) => {}
        other => panic!("expected OpenMarketplaceAddPrompt event, got {other:?}"),
    }

    chat.open_marketplace_add_prompt();
    let prompt = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        prompt.contains("owner/repo, git URL, or local marketplace path"),
        "expected marketplace source prompt, got:\n{prompt}"
    );

    chat.handle_paste("owner/repo".to_string());
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    match rx.try_recv() {
        Ok(AppEvent::OpenMarketplaceAddLoading { source }) => {
            assert_eq!(source, "owner/repo");
        }
        other => panic!("expected OpenMarketplaceAddLoading event, got {other:?}"),
    }
    match rx.try_recv() {
        Ok(AppEvent::FetchMarketplaceAdd {
            cwd: event_cwd,
            source,
        }) => {
            assert_eq!(event_cwd, cwd);
            assert_eq!(source, "owner/repo");
        }
        other => panic!("expected FetchMarketplaceAdd event, got {other:?}"),
    }
}

#[tokio::test]
async fn plugins_popup_upgrades_user_configured_git_marketplace_from_marketplace_tab() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let cwd = chat.config.cwd.to_path_buf();
    let temp = tempdir().expect("tempdir");
    let config_toml_path = temp.path().join("config.toml").abs();
    chat.config.config_layer_stack = ConfigLayerStack::default().with_user_config(
        &config_toml_path,
        toml::from_str::<TomlValue>(
            "[marketplaces.repo]\nsource_type = \"git\"\nsource = \"https://github.com/owner/repo.git\"\n",
        )
        .expect("marketplace config"),
    )
    .expect("marketplace user config should be valid");

    render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![
            plugins_test_curated_marketplace(Vec::new()),
            plugins_test_repo_marketplace(vec![plugins_test_summary(
                "plugin-debug",
                "debug",
                Some("Debug Plugin"),
                Some("Debug marketplace plugin."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            )]),
        ]),
    );

    while rx.try_recv().is_ok() {}
    let popup = select_plugins_tab_containing(&mut chat, /*width*/ 100, "Repo Marketplace.");
    assert!(
        popup.contains("Repo Marketplace.")
            && popup.contains("ctrl + u upgrade")
            && popup.contains("ctrl + r remove")
            && popup.contains("Debug Plugin"),
        "expected upgradeable user-configured marketplace tab, got:\n{popup}"
    );

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    chat.handle_key_event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));

    match rx.try_recv() {
        Ok(AppEvent::OpenMarketplaceUpgradeLoading { marketplace_name }) => {
            assert_eq!(marketplace_name, Some("repo".to_string()));
        }
        other => panic!("expected OpenMarketplaceUpgradeLoading event, got {other:?}"),
    }
    match rx.try_recv() {
        Ok(AppEvent::FetchMarketplaceUpgrade {
            cwd: event_cwd,
            marketplace_name,
        }) => {
            assert_eq!(event_cwd, cwd);
            assert_eq!(marketplace_name, Some("repo".to_string()));
        }
        other => panic!("expected FetchMarketplaceUpgrade event, got {other:?}"),
    }
    let no_more_events = rx.try_recv();
    assert!(
        no_more_events.is_err(),
        "expected no duplicate marketplace upgrade events, got {no_more_events:?}"
    );
}






#[tokio::test]
async fn plugins_popup_remote_row_opens_remote_detail() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let popup = render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![PluginMarketplaceEntry {
            name: "workspace-directory".to_string(),
            path: None,
            interface: Some(MarketplaceInterface {
                display_name: Some("Workspace".to_string()),
            }),
            plugins: vec![plugins_test_remote_summary(
                "plugins~Plugin_calendar",
                "calendar",
                Some("Calendar"),
                Some("Workspace schedules."),
                /*installed*/ false,
            )],
        }]),
    );
    let remote_row = popup
        .lines()
        .find(|line| line.contains("Calendar"))
        .expect("expected remote plugin row");
    assert!(
        remote_row.contains("Available")
            && remote_row.contains("Press Enter to install or view plugin details."),
        "expected remote plugin row to be viewable, got:\n{remote_row}"
    );

    while rx.try_recv().is_ok() {}
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    match rx.try_recv() {
        Ok(AppEvent::OpenPluginDetailLoading {
            plugin_display_name,
        }) => {
            assert_eq!(plugin_display_name, "Calendar");
        }
        other => panic!("expected OpenPluginDetailLoading event, got {other:?}"),
    }
    match rx.try_recv() {
        Ok(AppEvent::FetchPluginDetail { cwd: _, params }) => {
            assert_eq!(params.marketplace_path, None);
            assert_eq!(
                params.remote_marketplace_name,
                Some("workspace-directory".to_string())
            );
            assert_eq!(params.plugin_name, "plugins~Plugin_calendar");
        }
        other => panic!("expected FetchPluginDetail event, got {other:?}"),
    }
}

#[tokio::test]
async fn plugin_detail_unmaterialized_default_uses_remote_install_path() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let summary = PluginSummary {
        install_policy: PluginInstallPolicy::InstalledByDefault,
        ..plugins_test_remote_summary(
            "plugins~Plugin_linear",
            "linear",
            Some("Linear"),
            Some("Issue tracking."),
            /*installed*/ false,
        )
    };
    let cwd = chat.config.cwd.clone();
    chat.on_plugins_loaded(
        cwd.to_path_buf(),
        Ok(plugins_test_response(vec![PluginMarketplaceEntry {
            name: "workspace-shared-with-me-private".to_string(),
            path: None,
            interface: Some(MarketplaceInterface {
                display_name: Some("Shared with me".to_string()),
            }),
            plugins: vec![summary.clone()],
        }])),
    );
    chat.add_plugins_output();
    chat.on_plugin_detail_loaded(
        cwd.to_path_buf(),
        Ok(PluginReadResponse {
            plugin: PluginDetail {
                marketplace_name: "workspace-shared-with-me-private".to_string(),
                marketplace_path: None,
                summary,
                share_url: None,
                description: Some("Install shared Linear plugin.".to_string()),
                skills: Vec::new(),
                hooks: Vec::new(),
                apps: Vec::new(),
                app_templates: Vec::new(),
                mcp_servers: Vec::new(),
                scheduled_tasks: None,
            },
        }),
    );
    let popup = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        popup.contains("Install plugin") && popup.contains("Install this plugin now."),
        "expected remote detail to offer install, got:\n{popup}"
    );

    while rx.try_recv().is_ok() {}
    chat.handle_key_event(KeyEvent::from(KeyCode::Down));
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    match rx.try_recv() {
        Ok(AppEvent::OpenPluginInstallLoading {
            plugin_display_name,
        }) => {
            assert_eq!(plugin_display_name, "Linear");
        }
        other => panic!("expected OpenPluginInstallLoading event, got {other:?}"),
    }
    match rx.try_recv() {
        Ok(AppEvent::FetchPluginInstall {
            cwd: _,
            location: crate::app_event::PluginLocation::Remote { marketplace_name },
            plugin_name,
            plugin_display_name,
        }) => {
            assert_eq!(marketplace_name, "workspace-shared-with-me-private");
            assert_eq!(plugin_name, "plugins~Plugin_linear");
            assert_eq!(plugin_display_name, "Linear");
        }
        other => panic!("expected remote FetchPluginInstall event, got {other:?}"),
    }
}

#[tokio::test]
async fn plugin_detail_remote_uninstall_uses_remote_plugin_id() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let summary = plugins_test_remote_summary(
        "plugins~Plugin_linear",
        "linear",
        Some("Linear"),
        Some("Issue tracking."),
        /*installed*/ true,
    );
    let cwd = chat.config.cwd.clone();
    chat.on_plugins_loaded(
        cwd.to_path_buf(),
        Ok(plugins_test_response(vec![
            plugins_test_remote_marketplace(
                "workspace-shared-with-me-private",
                "Shared with me",
                vec![summary.clone()],
            ),
        ])),
    );
    chat.add_plugins_output();
    chat.on_plugin_detail_loaded(
        cwd.to_path_buf(),
        Ok(PluginReadResponse {
            plugin: plugins_test_remote_detail(
                "workspace-shared-with-me-private",
                summary,
                Some("Installed shared Linear plugin."),
            ),
        }),
    );

    while rx.try_recv().is_ok() {}
    chat.handle_key_event(KeyEvent::from(KeyCode::Down));
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    match rx.try_recv() {
        Ok(AppEvent::OpenPluginUninstallLoading {
            plugin_display_name,
        }) => {
            assert_eq!(plugin_display_name, "Linear");
        }
        other => panic!("expected OpenPluginUninstallLoading event, got {other:?}"),
    }
    match rx.try_recv() {
        Ok(AppEvent::FetchPluginUninstall {
            plugin_id,
            plugin_display_name,
            ..
        }) => {
            assert_eq!(plugin_id, "plugins~Plugin_linear");
            assert_eq!(plugin_display_name, "Linear");
        }
        other => panic!("expected remote FetchPluginUninstall event, got {other:?}"),
    }
}

#[tokio::test]
async fn plugin_detail_remote_without_remote_id_disables_uninstall_action() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let summary = PluginSummary {
        source: PluginSource::Remote,
        ..plugins_test_summary(
            "linear@workspace-shared-with-me-private",
            "linear",
            Some("Linear"),
            Some("Issue tracking."),
            /*installed*/ true,
            /*enabled*/ true,
            PluginInstallPolicy::Available,
        )
    };
    let cwd = chat.config.cwd.clone();
    chat.on_plugins_loaded(
        cwd.to_path_buf(),
        Ok(plugins_test_response(vec![
            plugins_test_remote_marketplace(
                "workspace-shared-with-me-private",
                "Shared with me",
                vec![summary.clone()],
            ),
        ])),
    );
    chat.add_plugins_output();
    chat.on_plugin_detail_loaded(
        cwd.to_path_buf(),
        Ok(PluginReadResponse {
            plugin: plugins_test_remote_detail(
                "workspace-shared-with-me-private",
                summary,
                Some("Installed shared Linear plugin."),
            ),
        }),
    );

    let popup = render_bottom_popup(&chat, /*width*/ 120);
    assert!(
        popup.contains("This remote plugin did not provide an uninstall identity.")
            && !popup.contains("Remove this plugin now."),
        "expected missing remote ID to disable uninstall, got:\n{popup}"
    );

    chat.handle_key_event(KeyEvent::from(KeyCode::Down));
    assert_eq!(
        render_bottom_popup(&chat, /*width*/ 120),
        popup,
        "expected navigation to skip the disabled uninstall row"
    );
}




#[tokio::test]
async fn plugins_popup_admin_disabled_available_plugin_has_view_only_hint() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let summary = PluginSummary {
        availability: PluginAvailability::DisabledByAdmin,
        ..plugins_test_summary(
            "plugin-admin-blocked",
            "admin-blocked",
            Some("Admin Blocked"),
            Some("Blocked by policy."),
            /*installed*/ false,
            /*enabled*/ true,
            PluginInstallPolicy::Available,
        )
    };
    render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![plugins_test_curated_marketplace(vec![summary])]),
    );

    let popup = render_bottom_popup(&chat, /*width*/ 100);
    let admin_blocked_row = popup
        .lines()
        .find(|line| line.contains("Admin Blocked"))
        .expect("expected admin-disabled plugin row");
    assert!(
        admin_blocked_row.contains("Press Enter to view plugin details.")
            && !admin_blocked_row.contains("install or view"),
        "expected admin-disabled available plugin to stay view-only, got:\n{admin_blocked_row}"
    );
}


#[tokio::test]
async fn plugins_popup_remote_detail_tracks_physical_and_policy_install_state() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let remote_plugin_id = "plugins~Plugin_docs";
    let remote_marketplace_name = "workspace-shared-with-me-private";
    let mut local_summary = PluginSummary {
        share_context: Some(PluginShareContext {
            remote_plugin_id: remote_plugin_id.to_string(),
            remote_version: None,
            discoverability: None,
            share_url: None,
            creator_account_user_id: None,
            creator_name: None,
            share_principals: None,
            can_publish_to_workspace: None,
        }),
        ..plugins_test_summary(
            "plugin-docs",
            "docs",
            Some("Docs"),
            Some("Local editable docs plugin."),
            /*installed*/ false,
            /*enabled*/ true,
            PluginInstallPolicy::InstalledByDefault,
        )
    };
    let popup = render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![
            plugins_test_curated_marketplace(vec![local_summary.clone()]),
            PluginMarketplaceEntry {
                name: remote_marketplace_name.to_string(),
                path: None,
                interface: Some(MarketplaceInterface {
                    display_name: Some("Shared with me".to_string()),
                }),
                plugins: vec![plugins_test_remote_summary(
                    remote_plugin_id,
                    "docs",
                    Some("Docs"),
                    Some("Shared docs plugin."),
                    /*installed*/ true,
                )],
            },
        ]),
    );
    let all_plugins_row = popup
        .lines()
        .find(|line| line.contains("Docs"))
        .expect("expected all-plugins row");
    assert!(
        popup.contains("Installed 1 of 1 available plugins.")
            && all_plugins_row.contains("Installed")
            && !all_plugins_row.contains("Available"),
        "expected installed remote duplicate to win over local mapped share, got:\n{popup}"
    );

    chat.handle_key_event(KeyEvent::from(KeyCode::Right));
    let installed_popup = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        installed_popup.contains("Showing 1 installed plugins.")
            && installed_popup.contains("Docs"),
        "expected installed remote duplicate in the Installed tab, got:\n{installed_popup}"
    );

    while rx.try_recv().is_ok() {}
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    match rx.try_recv() {
        Ok(AppEvent::OpenPluginDetailLoading {
            plugin_display_name,
        }) => {
            assert_eq!(plugin_display_name, "Docs");
        }
        other => panic!("expected OpenPluginDetailLoading event, got {other:?}"),
    }
    match rx.try_recv() {
        Ok(AppEvent::FetchPluginDetail { params, .. }) => {
            assert_eq!(params.marketplace_path, None);
            assert_eq!(
                params.remote_marketplace_name,
                Some(remote_marketplace_name.to_string())
            );
            assert_eq!(params.plugin_name, remote_plugin_id);
        }
        other => panic!("expected FetchPluginDetail event, got {other:?}"),
    }

    local_summary.install_policy = PluginInstallPolicy::Available;
    let mut remote_summary = plugins_test_remote_summary(
        remote_plugin_id,
        "docs",
        Some("Docs"),
        Some("Shared docs plugin."),
        /*installed*/ false,
    );
    remote_summary.install_policy = PluginInstallPolicy::InstalledByDefault;
    let popup = render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![
            plugins_test_curated_marketplace(vec![local_summary]),
            PluginMarketplaceEntry {
                name: remote_marketplace_name.to_string(),
                path: None,
                interface: Some(MarketplaceInterface {
                    display_name: Some("Shared with me".to_string()),
                }),
                plugins: vec![remote_summary],
            },
        ]),
    );
    assert!(
        popup.contains("Installed 0 of 1 available plugins.") && popup.contains("Admin assigned"),
        "expected the unmaterialized admin-assigned remote duplicate to win without counting as installed, got:\n{popup}"
    );

    chat.handle_key_event(KeyEvent::from(KeyCode::Right));
    let installed_popup = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        installed_popup.contains("Showing 0 installed plugins.")
            && installed_popup.contains("No installed plugins")
            && !installed_popup.contains("Docs"),
        "expected the unmaterialized admin-assigned plugin to stay out of the Installed tab, got:\n{installed_popup}"
    );
    chat.handle_key_event(KeyEvent::from(KeyCode::Left));

    while rx.try_recv().is_ok() {}
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    assert!(matches!(
        rx.try_recv(),
        Ok(AppEvent::OpenPluginDetailLoading { .. })
    ));
    match rx.try_recv() {
        Ok(AppEvent::FetchPluginDetail { params, .. }) => {
            assert_eq!(params.marketplace_path, None);
            assert_eq!(
                params.remote_marketplace_name,
                Some(remote_marketplace_name.to_string())
            );
            assert_eq!(params.plugin_name, remote_plugin_id);
        }
        other => panic!("expected FetchPluginDetail event, got {other:?}"),
    }
}


#[tokio::test]
async fn plugins_popup_refresh_preserves_selected_row_position() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let initial = plugins_test_response(vec![plugins_test_curated_marketplace(vec![
        plugins_test_summary(
            "plugin-notion",
            "notion",
            Some("Notion"),
            Some("Workspace docs."),
            /*installed*/ false,
            /*enabled*/ true,
            PluginInstallPolicy::Available,
        ),
        plugins_test_summary(
            "plugin-slack",
            "slack",
            Some("Slack"),
            Some("Team chat."),
            /*installed*/ false,
            /*enabled*/ true,
            PluginInstallPolicy::Available,
        ),
    ])]);
    render_loaded_plugins_popup(&mut chat, initial);
    chat.handle_key_event(KeyEvent::from(KeyCode::Down));

    let before = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        before.contains("› [-] Slack"),
        "expected Slack to be selected before refresh, got:\n{before}"
    );

    let refreshed = plugins_test_response(vec![plugins_test_curated_marketplace(vec![
        plugins_test_summary(
            "plugin-airtable",
            "airtable",
            Some("Airtable"),
            Some("Structured records."),
            /*installed*/ false,
            /*enabled*/ true,
            PluginInstallPolicy::Available,
        ),
        plugins_test_summary(
            "plugin-notion",
            "notion",
            Some("Notion"),
            Some("Workspace docs."),
            /*installed*/ false,
            /*enabled*/ true,
            PluginInstallPolicy::Available,
        ),
        plugins_test_summary(
            "plugin-slack",
            "slack",
            Some("Slack"),
            Some("Team chat."),
            /*installed*/ false,
            /*enabled*/ true,
            PluginInstallPolicy::Available,
        ),
    ])]);
    let cwd = chat.config.cwd.clone();
    chat.on_plugins_loaded(cwd.to_path_buf(), Ok(refreshed));

    let after = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        after.contains("› [-] Notion"),
        "expected refresh to preserve the selected row position, got:\n{after}"
    );
    assert!(
        after.contains("Airtable"),
        "expected refreshed popup to include the updated plugin list, got:\n{after}"
    );
    assert!(
        after.contains("Slack"),
        "expected refreshed popup to include the updated plugin list, got:\n{after}"
    );
}

#[tokio::test]
async fn plugins_popup_refreshes_installed_counts_after_install() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let initial = plugins_test_response(vec![plugins_test_curated_marketplace(vec![
        plugins_test_summary(
            "plugin-calendar",
            "calendar",
            Some("Calendar"),
            Some("Schedule management."),
            /*installed*/ false,
            /*enabled*/ true,
            PluginInstallPolicy::Available,
        ),
        plugins_test_summary(
            "plugin-drive",
            "drive",
            Some("Drive"),
            Some("Document access."),
            /*installed*/ true,
            /*enabled*/ true,
            PluginInstallPolicy::Available,
        ),
    ])]);
    let before = render_loaded_plugins_popup(&mut chat, initial);
    assert!(
        before.contains("Installed 1 of 2 available plugins."),
        "expected initial installed count before refresh, got:\n{before}"
    );
    assert!(
        before.contains("Available"),
        "expected pre-install popup copy before refresh, got:\n{before}"
    );

    let refreshed = plugins_test_response(vec![plugins_test_curated_marketplace(vec![
        plugins_test_summary(
            "plugin-calendar",
            "calendar",
            Some("Calendar"),
            Some("Schedule management."),
            /*installed*/ true,
            /*enabled*/ true,
            PluginInstallPolicy::Available,
        ),
        plugins_test_summary(
            "plugin-drive",
            "drive",
            Some("Drive"),
            Some("Document access."),
            /*installed*/ true,
            /*enabled*/ true,
            PluginInstallPolicy::Available,
        ),
    ])]);
    let cwd = chat.config.cwd.clone();
    chat.on_plugins_loaded(cwd.to_path_buf(), Ok(refreshed));

    let after = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        after.contains("Installed 2 of 2 available plugins."),
        "expected /plugins to refresh installed counts after install, got:\n{after}"
    );
    assert!(
        after.contains("Installed   Space to disable; Enter view details."),
        "expected refreshed selected row copy to reflect the installed plugin state, got:\n{after}"
    );
}

#[tokio::test]
async fn plugins_popup_space_toggles_installed_plugin_from_list() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let cwd = chat.config.cwd.to_path_buf();
    render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![plugins_test_curated_marketplace(vec![
            plugins_test_summary(
                "plugin-calendar",
                "calendar",
                Some("Calendar"),
                Some("Schedule management."),
                /*installed*/ true,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
            plugins_test_summary(
                "plugin-drive",
                "drive",
                Some("Drive"),
                Some("Document access."),
                /*installed*/ true,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
        ])]),
    );

    while rx.try_recv().is_ok() {}
    chat.handle_key_event(KeyEvent::from(KeyCode::Down));
    chat.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

    match rx.try_recv() {
        Ok(AppEvent::SetPluginEnabled {
            cwd: event_cwd,
            plugin_id,
            enabled,
        }) => {
            assert_eq!(event_cwd, cwd);
            assert_eq!(plugin_id, "plugin-drive");
            assert!(!enabled);
        }
        other => panic!("expected SetPluginEnabled event, got {other:?}"),
    }

    chat.on_plugin_enabled_set(
        cwd,
        "plugin-drive".to_string(),
        /*enabled*/ false,
        Ok(()),
    );

    let popup = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        popup.contains("› [ ] Drive"),
        "expected selected plugin row to stay selected after refresh, got:\n{popup}"
    );
}

#[tokio::test]
async fn plugins_popup_space_on_uninstalled_row_does_not_start_search() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![plugins_test_curated_marketplace(vec![
            plugins_test_summary(
                "plugin-calendar",
                "calendar",
                Some("Calendar"),
                Some("Schedule management."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
            plugins_test_summary(
                "plugin-drive",
                "drive",
                Some("Drive"),
                Some("Document access."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
        ])]),
    );

    while rx.try_recv().is_ok() {}
    let before = render_bottom_popup(&chat, /*width*/ 100);
    chat.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    let after = render_bottom_popup(&chat, /*width*/ 100);

    assert!(
        rx.try_recv().is_err(),
        "did not expect Space on an uninstalled plugin to emit an event"
    );
    assert_eq!(after, before);
}

#[tokio::test]
async fn plugins_popup_space_with_active_search_does_not_toggle_installed_plugin() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![plugins_test_curated_marketplace(vec![
            plugins_test_summary(
                "plugin-calendar",
                "calendar",
                Some("Calendar"),
                Some("Schedule management."),
                /*installed*/ true,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
            plugins_test_summary(
                "plugin-drive",
                "drive",
                Some("Drive"),
                Some("Document access."),
                /*installed*/ true,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
        ])]),
    );

    while rx.try_recv().is_ok() {}
    chat.handle_key_event(KeyEvent::from(KeyCode::Down));
    type_plugins_search_query(&mut chat, "dr");
    chat.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

    assert!(
        rx.try_recv().is_err(),
        "did not expect Space with an active plugin search to emit a toggle event"
    );
}


#[tokio::test]
async fn plugins_popup_search_matches_plugin_descriptions() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![plugins_test_curated_marketplace(vec![
            plugins_test_summary(
                "plugin-calendar",
                "calendar",
                Some("Calendar"),
                Some("Schedule management."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
            plugins_test_summary(
                "plugin-drive",
                "drive",
                Some("Drive"),
                Some("Document access."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
        ])]),
    );

    type_plugins_search_query(&mut chat, "document");

    let popup = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        popup.contains("Drive") && !popup.contains("Calendar"),
        "expected plugin search to match descriptions, got:\n{popup}"
    );
}

#[tokio::test]
async fn plugins_popup_installed_tab_filters_rows_and_clears_search() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![plugins_test_curated_marketplace(vec![
            plugins_test_summary(
                "plugin-calendar",
                "calendar",
                Some("Calendar"),
                Some("Schedule management."),
                /*installed*/ true,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
            plugins_test_summary(
                "plugin-slack",
                "slack",
                Some("Slack"),
                Some("Team chat."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
        ])]),
    );

    type_plugins_search_query(&mut chat, "sla");
    chat.handle_key_event(KeyEvent::from(KeyCode::Right));

    let popup = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        popup.contains("Installed plugins.") && popup.contains("Showing 1 installed plugins."),
        "expected Installed tab header, got:\n{popup}"
    );
    assert!(
        popup.contains("Calendar") && !popup.contains("Slack"),
        "expected Installed tab to show only installed plugins, got:\n{popup}"
    );
    assert!(
        !popup.contains("sla"),
        "expected tab switch to clear search query, got:\n{popup}"
    );
}

#[tokio::test]
async fn plugins_popup_test_marketplace_tab_omits_marketplace_in_rows() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![
            plugins_test_curated_marketplace(vec![plugins_test_summary(
                "plugin-calendar",
                "calendar",
                Some("Calendar"),
                Some("Schedule management."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            )]),
            plugins_test_repo_marketplace(vec![plugins_test_summary(
                "plugin-repo",
                "repo",
                Some("Repo Plugin"),
                Some("Repo-only plugin."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            )]),
        ]),
    );

    chat.handle_key_event(KeyEvent::from(KeyCode::Right));
    chat.handle_key_event(KeyEvent::from(KeyCode::Right));

    let popup = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        popup.contains("Test Marketplace marketplace."),
        "expected Test Marketplace tab header, got:\n{popup}"
    );
    assert!(
        popup.contains("Calendar") && !popup.contains("Repo Plugin"),
        "expected Test Marketplace tab to show only official marketplace plugins, got:\n{popup}"
    );
    assert!(
        !popup.contains("Test Marketplace ·"),
        "expected marketplace-specific rows to omit marketplace labels, got:\n{popup}"
    );
}

#[tokio::test]
async fn plugins_popup_refresh_preserves_duplicate_marketplace_tab_by_path() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    let response = plugins_test_response(vec![
        PluginMarketplaceEntry {
            name: "duplicate".to_string(),
            path: Some(plugins_test_absolute_path(
                "marketplaces/home/marketplace.json",
            )),
            interface: Some(MarketplaceInterface {
                display_name: Some("Duplicate Marketplace".to_string()),
            }),
            plugins: vec![plugins_test_summary(
                "plugin-home",
                "home",
                Some("Home Plugin"),
                Some("Home marketplace plugin."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            )],
        },
        PluginMarketplaceEntry {
            name: "duplicate".to_string(),
            path: Some(plugins_test_absolute_path(
                "marketplaces/repo/marketplace.json",
            )),
            interface: Some(MarketplaceInterface {
                display_name: Some("Duplicate Marketplace".to_string()),
            }),
            plugins: vec![plugins_test_summary(
                "plugin-repo",
                "repo",
                Some("Repo Plugin"),
                Some("Repo marketplace plugin."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            )],
        },
    ]);
    let cwd = chat.config.cwd.to_path_buf();
    chat.on_plugins_loaded(cwd.clone(), Ok(response.clone()));
    chat.add_plugins_output();

    for _ in 0..10 {
        let popup = render_bottom_popup(&chat, /*width*/ 100);
        if popup.contains("Duplicate Marketplace (2/2).")
            && popup.contains("Repo Plugin")
            && !popup.contains("Home Plugin")
        {
            break;
        }
        chat.handle_key_event(KeyEvent::from(KeyCode::Right));
    }

    chat.on_plugins_loaded(cwd, Ok(response));

    let popup = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        popup.contains("Duplicate Marketplace (2/2)."),
        "expected refresh to preserve the second duplicate marketplace tab, got:\n{popup}"
    );
    assert!(
        popup.contains("Repo Plugin") && !popup.contains("Home Plugin"),
        "expected second duplicate marketplace rows after refresh, got:\n{popup}"
    );
}

#[tokio::test]
async fn plugins_popup_search_no_matches_and_backspace_restores_results() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_feature_enabled(Feature::Plugins, /*enabled*/ true);

    render_loaded_plugins_popup(
        &mut chat,
        plugins_test_response(vec![plugins_test_curated_marketplace(vec![
            plugins_test_summary(
                "plugin-calendar",
                "calendar",
                Some("Calendar"),
                Some("Schedule management."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
            plugins_test_summary(
                "plugin-slack",
                "slack",
                Some("Slack"),
                Some("Team chat."),
                /*installed*/ false,
                /*enabled*/ true,
                PluginInstallPolicy::Available,
            ),
        ])]),
    );

    type_plugins_search_query(&mut chat, "zzz");

    let no_matches = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        no_matches.contains("zzz"),
        "expected popup to show the typed search query, got:\n{no_matches}"
    );
    assert!(
        no_matches.contains("no matches"),
        "expected popup to render the no-matches UX, got:\n{no_matches}"
    );

    for _ in 0..3 {
        chat.handle_key_event(KeyEvent::from(KeyCode::Backspace));
    }

    let restored = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        restored.contains("Calendar") && restored.contains("Slack"),
        "expected clearing the query to restore the plugin rows, got:\n{restored}"
    );
    assert!(
        !restored.contains("no matches"),
        "did not expect the no-matches state after clearing the query, got:\n{restored}"
    );
}



#[tokio::test]
async fn apps_installed_mentions_revoke_access_and_reject_stale_account_results() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    set_managed_auth(&mut chat);
    chat.set_feature_enabled(Feature::Apps, /*enabled*/ true);
    let generation = chat.connector_scope_generation();
    let connector_id = "account-app";
    let connector =
        serde_json::from_str(r#"{"id":"account-app","name":"Account app","isAccessible":true}"#)
            .expect("valid installed app");
    let snapshot = ConnectorsSnapshot {
        connectors: vec![connector],
    };
    chat.on_connector_mentions_loaded(generation, Ok(snapshot.clone()));
    assert!(chat.connectors.installed_app_ids.contains(connector_id));

    chat.refresh_connector_mentions(/*force_refresh*/ false);
    rx.try_recv().expect("pre-disable mention refresh");
    chat.update_connector_enabled(connector_id, /*enabled*/ false);
    chat.on_connector_mentions_loaded(generation, Ok(snapshot.clone()));
    assert_eq!(chat.connectors_for_mentions(), Some([].as_slice()));
    rx.try_recv().expect("post-disable mention refresh");
    chat.on_connector_mentions_loaded(generation, Ok(snapshot.clone()));
    assert_eq!(
        chat.connectors_for_mentions(),
        Some(snapshot.connectors.as_slice())
    );

    chat.on_connectors_loaded(Ok(snapshot.clone()), /*is_final*/ true);
    for enabled in [false, true] {
        chat.update_connector_enabled(connector_id, enabled);
        assert_eq!(chat.connectors_for_mentions(), Some([].as_slice()));
        assert_matches!(
            rx.try_recv(),
            Ok(AppEvent::FetchInstalledConnectorMentions { force_refresh, .. })
                if force_refresh == enabled
        );
        let mut refreshed = snapshot.clone();
        refreshed.connectors[0].is_enabled = enabled;
        chat.on_connector_mentions_loaded(generation, Ok(refreshed));
    }

    chat.update_account_state(
        /*status_account_display*/ None, /*plan_type*/ None,
        /*has_managed_account*/ true, /*has_codex_backend_auth*/ true,
    );

    assert_ne!(chat.connector_scope_generation(), generation);
    assert_matches!(&chat.connectors.cache, ConnectorsCacheState::Uninitialized);
    assert!(chat.connectors_for_mentions().is_none());
    assert!(chat.connectors.mention_refresh_in_flight);

    chat.on_connector_mentions_loaded(generation, Ok(snapshot));
    assert!(chat.connectors_for_mentions().is_none());
    assert!(chat.connectors.mention_refresh_in_flight);
    assert!(chat.connectors.installed_app_ids.is_empty());

    let (mut replacement, _, _) = make_chatwidget_manual(/*model_override*/ None).await;
    replacement.invalidate_connector_scope();
    assert_ne!(
        chat.connector_scope_generation(),
        replacement.connector_scope_generation()
    );
}

#[tokio::test]
async fn apps_refresh_failure_keeps_existing_full_snapshot() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    set_managed_auth(&mut chat);
    chat.config
        .features
        .enable(Feature::Apps)
        .expect("test config should allow feature update");
    chat.bottom_pane.set_connectors_enabled(/*enabled*/ true);
    let notion_id = "unit_test_apps_refresh_failure_connector_1";
    let linear_id = "unit_test_apps_refresh_failure_connector_2";

    let full_connectors = vec![
        AppInfo {
            id: notion_id.to_string(),
            name: "Notion".to_string(),
            description: Some("Workspace docs".to_string()),
            logo_url: None,
            logo_url_dark: None,
            icon_assets: None,
            icon_dark_assets: None,
            distribution_channel: None,
            branding: None,
            app_metadata: None,
            labels: None,
            install_url: Some("https://example.test/notion".to_string()),
            is_accessible: true,
            is_enabled: true,
            plugin_display_names: Vec::new(),
        },
        AppInfo {
            id: linear_id.to_string(),
            name: "Linear".to_string(),
            description: Some("Project tracking".to_string()),
            logo_url: None,
            logo_url_dark: None,
            icon_assets: None,
            icon_dark_assets: None,
            distribution_channel: None,
            branding: None,
            app_metadata: None,
            labels: None,
            install_url: Some("https://example.test/linear".to_string()),
            is_accessible: false,
            is_enabled: true,
            plugin_display_names: Vec::new(),
        },
    ];
    chat.on_connectors_loaded(
        Ok(ConnectorsSnapshot {
            connectors: full_connectors.clone(),
        }),
        /*is_final*/ true,
    );

    chat.on_connectors_loaded(
        Ok(ConnectorsSnapshot {
            connectors: vec![AppInfo {
                id: notion_id.to_string(),
                name: "Notion".to_string(),
                description: Some("Workspace docs".to_string()),
                logo_url: None,
                logo_url_dark: None,
                icon_assets: None,
                icon_dark_assets: None,
                distribution_channel: None,
                branding: None,
                app_metadata: None,
                labels: None,
                install_url: Some("https://example.test/notion".to_string()),
                is_accessible: true,
                is_enabled: true,
                plugin_display_names: Vec::new(),
            }],
        }),
        /*is_final*/ false,
    );
    chat.on_connectors_loaded(
        Err("failed to load apps".to_string()),
        /*is_final*/ true,
    );

    assert_matches!(
        &chat.connectors.cache,
        ConnectorsCacheState::Ready(snapshot) if snapshot.connectors == full_connectors
    );

    chat.add_connectors_output();
    let popup = render_bottom_popup(&chat, /*width*/ 80);
    assert!(
        popup.contains("Installed 1 of 2 available apps."),
        "expected previous full snapshot to be preserved, got:\n{popup}"
    );
}

#[tokio::test]
async fn apps_popup_preserves_selected_app_across_refresh() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    set_managed_auth(&mut chat);
    chat.config
        .features
        .enable(Feature::Apps)
        .expect("test config should allow feature update");
    chat.bottom_pane.set_connectors_enabled(/*enabled*/ true);

    chat.on_connectors_loaded(
        Ok(ConnectorsSnapshot {
            connectors: vec![
                AppInfo {
                    id: "notion".to_string(),
                    name: "Notion".to_string(),
                    description: Some("Workspace docs".to_string()),
                    logo_url: None,
                    logo_url_dark: None,
                    icon_assets: None,
                    icon_dark_assets: None,
                    distribution_channel: None,
                    branding: None,
                    app_metadata: None,
                    labels: None,
                    install_url: Some("https://example.test/notion".to_string()),
                    is_accessible: true,
                    is_enabled: true,
                    plugin_display_names: Vec::new(),
                },
                AppInfo {
                    id: "slack".to_string(),
                    name: "Slack".to_string(),
                    description: Some("Team chat".to_string()),
                    logo_url: None,
                    logo_url_dark: None,
                    icon_assets: None,
                    icon_dark_assets: None,
                    distribution_channel: None,
                    branding: None,
                    app_metadata: None,
                    labels: None,
                    install_url: Some("https://example.test/slack".to_string()),
                    is_accessible: true,
                    is_enabled: true,
                    plugin_display_names: Vec::new(),
                },
            ],
        }),
        /*is_final*/ true,
    );
    chat.add_connectors_output();
    chat.handle_key_event(KeyEvent::from(KeyCode::Down));

    let before = render_bottom_popup(&chat, /*width*/ 80);
    assert!(
        before.contains("› Slack"),
        "expected Slack to be selected before refresh, got:\n{before}"
    );

    chat.on_connectors_loaded(
        Ok(ConnectorsSnapshot {
            connectors: vec![
                AppInfo {
                    id: "airtable".to_string(),
                    name: "Airtable".to_string(),
                    description: Some("Spreadsheets".to_string()),
                    logo_url: None,
                    logo_url_dark: None,
                    icon_assets: None,
                    icon_dark_assets: None,
                    distribution_channel: None,
                    branding: None,
                    app_metadata: None,
                    labels: None,
                    install_url: Some("https://example.test/airtable".to_string()),
                    is_accessible: true,
                    is_enabled: true,
                    plugin_display_names: Vec::new(),
                },
                AppInfo {
                    id: "notion".to_string(),
                    name: "Notion".to_string(),
                    description: Some("Workspace docs".to_string()),
                    logo_url: None,
                    logo_url_dark: None,
                    icon_assets: None,
                    icon_dark_assets: None,
                    distribution_channel: None,
                    branding: None,
                    app_metadata: None,
                    labels: None,
                    install_url: Some("https://example.test/notion".to_string()),
                    is_accessible: true,
                    is_enabled: true,
                    plugin_display_names: Vec::new(),
                },
                AppInfo {
                    id: "slack".to_string(),
                    name: "Slack".to_string(),
                    description: Some("Team chat".to_string()),
                    logo_url: None,
                    logo_url_dark: None,
                    icon_assets: None,
                    icon_dark_assets: None,
                    distribution_channel: None,
                    branding: None,
                    app_metadata: None,
                    labels: None,
                    install_url: Some("https://example.test/slack".to_string()),
                    is_accessible: true,
                    is_enabled: true,
                    plugin_display_names: Vec::new(),
                },
            ],
        }),
        /*is_final*/ true,
    );

    let after = render_bottom_popup(&chat, /*width*/ 80);
    assert!(
        after.contains("› Slack"),
        "expected Slack to stay selected after refresh, got:\n{after}"
    );
    assert!(
        !after.contains("› Notion"),
        "did not expect selection to reset to Notion after refresh, got:\n{after}"
    );
}

#[tokio::test]
async fn apps_refresh_failure_with_cached_snapshot_triggers_pending_force_refetch() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    set_managed_auth(&mut chat);
    chat.config
        .features
        .enable(Feature::Apps)
        .expect("test config should allow feature update");
    chat.bottom_pane.set_connectors_enabled(/*enabled*/ true);
    chat.connectors.prefetch_in_flight = true;
    chat.connectors.force_refetch_pending = true;

    let full_connectors = vec![AppInfo {
        id: "unit_test_apps_refresh_failure_pending_connector".to_string(),
        name: "Notion".to_string(),
        description: Some("Workspace docs".to_string()),
        logo_url: None,
        logo_url_dark: None,
        icon_assets: None,
        icon_dark_assets: None,
        distribution_channel: None,
        branding: None,
        app_metadata: None,
        labels: None,
        install_url: Some("https://example.test/notion".to_string()),
        is_accessible: true,
        is_enabled: true,
        plugin_display_names: Vec::new(),
    }];
    chat.connectors.cache = ConnectorsCacheState::Ready(ConnectorsSnapshot {
        connectors: full_connectors.clone(),
    });

    chat.on_connectors_loaded(
        Err("failed to load apps".to_string()),
        /*is_final*/ true,
    );

    assert!(chat.connectors.prefetch_in_flight);
    assert!(!chat.connectors.force_refetch_pending);
    assert_matches!(
        &chat.connectors.cache,
        ConnectorsCacheState::Ready(snapshot) if snapshot.connectors == full_connectors
    );
}

#[tokio::test]
async fn apps_popup_keeps_existing_full_snapshot_while_partial_refresh_loads() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    set_managed_auth(&mut chat);
    chat.config
        .features
        .enable(Feature::Apps)
        .expect("test config should allow feature update");
    chat.bottom_pane.set_connectors_enabled(/*enabled*/ true);

    let full_connectors = vec![
        AppInfo {
            id: "unit_test_connector_1".to_string(),
            name: "Notion".to_string(),
            description: Some("Workspace docs".to_string()),
            logo_url: None,
            logo_url_dark: None,
            icon_assets: None,
            icon_dark_assets: None,
            distribution_channel: None,
            branding: None,
            app_metadata: None,
            labels: None,
            install_url: Some("https://example.test/notion".to_string()),
            is_accessible: true,
            is_enabled: true,
            plugin_display_names: Vec::new(),
        },
        AppInfo {
            id: "unit_test_connector_2".to_string(),
            name: "Linear".to_string(),
            description: Some("Project tracking".to_string()),
            logo_url: None,
            logo_url_dark: None,
            icon_assets: None,
            icon_dark_assets: None,
            distribution_channel: None,
            branding: None,
            app_metadata: None,
            labels: None,
            install_url: Some("https://example.test/linear".to_string()),
            is_accessible: false,
            is_enabled: true,
            plugin_display_names: Vec::new(),
        },
    ];
    chat.on_connectors_loaded(
        Ok(ConnectorsSnapshot {
            connectors: full_connectors.clone(),
        }),
        /*is_final*/ true,
    );
    chat.add_connectors_output();

    chat.on_connectors_loaded(
        Ok(ConnectorsSnapshot {
            connectors: vec![
                AppInfo {
                    id: "unit_test_connector_1".to_string(),
                    name: "Notion".to_string(),
                    description: Some("Workspace docs".to_string()),
                    logo_url: None,
                    logo_url_dark: None,
                    icon_assets: None,
                    icon_dark_assets: None,
                    distribution_channel: None,
                    branding: None,
                    app_metadata: None,
                    labels: None,
                    install_url: Some("https://example.test/notion".to_string()),
                    is_accessible: true,
                    is_enabled: true,
                    plugin_display_names: Vec::new(),
                },
                AppInfo {
                    id: "connector_provider_hidden".to_string(),
                    name: "Hidden provider".to_string(),
                    description: Some("Should be filtered".to_string()),
                    logo_url: None,
                    logo_url_dark: None,
                    icon_assets: None,
                    icon_dark_assets: None,
                    distribution_channel: None,
                    branding: None,
                    app_metadata: None,
                    labels: None,
                    install_url: Some("https://example.test/hidden-provider".to_string()),
                    is_accessible: true,
                    is_enabled: true,
                    plugin_display_names: Vec::new(),
                },
            ],
        }),
        /*is_final*/ false,
    );

    assert_matches!(
        &chat.connectors.cache,
        ConnectorsCacheState::Ready(snapshot) if snapshot.connectors == full_connectors
    );

    let popup = render_bottom_popup(&chat, /*width*/ 80);
    assert!(
        popup.contains("Installed 1 of 2 available apps."),
        "expected popup to keep the last full snapshot while partial refresh loads, got:\n{popup}"
    );
    assert!(
        !popup.contains("Hidden provider"),
        "expected popup to ignore partial refresh rows until the full list arrives, got:\n{popup}"
    );
}


#[tokio::test]
async fn apps_refresh_failure_without_full_snapshot_falls_back_to_installed_apps() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    set_managed_auth(&mut chat);
    chat.config
        .features
        .enable(Feature::Apps)
        .expect("test config should allow feature update");
    chat.bottom_pane.set_connectors_enabled(/*enabled*/ true);

    chat.on_connectors_loaded(
        Ok(ConnectorsSnapshot {
            connectors: vec![AppInfo {
                id: "unit_test_apps_refresh_failure_fallback_connector".to_string(),
                name: "Notion".to_string(),
                description: Some("Workspace docs".to_string()),
                logo_url: None,
                logo_url_dark: None,
                icon_assets: None,
                icon_dark_assets: None,
                distribution_channel: None,
                branding: None,
                app_metadata: None,
                labels: None,
                install_url: Some("https://example.test/notion".to_string()),
                is_accessible: true,
                is_enabled: true,
                plugin_display_names: Vec::new(),
            }],
        }),
        /*is_final*/ false,
    );

    chat.add_connectors_output();
    let loading_popup = render_bottom_popup(&chat, /*width*/ 80);
    assert!(
        loading_popup.contains("Loading installed and available apps..."),
        "expected /apps to keep showing loading before the final result, got:\n{loading_popup}"
    );

    chat.on_connectors_loaded(
        Err("failed to load apps".to_string()),
        /*is_final*/ true,
    );

    assert_matches!(
        &chat.connectors.cache,
        ConnectorsCacheState::Ready(snapshot) if snapshot.connectors.len() == 1
    );

    let popup = render_bottom_popup(&chat, /*width*/ 80);
    assert!(
        popup.contains("Installed 1 of 1 available apps."),
        "expected /apps to fall back to the installed apps snapshot, got:\n{popup}"
    );
    assert!(
        popup.contains("Installed. Press Enter to open the app page"),
        "expected the fallback popup to behave like the installed apps view, got:\n{popup}"
    );
}

#[tokio::test]
async fn apps_popup_shows_disabled_status_for_installed_but_disabled_apps() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    set_managed_auth(&mut chat);
    chat.config
        .features
        .enable(Feature::Apps)
        .expect("test config should allow feature update");
    chat.bottom_pane.set_connectors_enabled(/*enabled*/ true);

    chat.on_connectors_loaded(
        Ok(ConnectorsSnapshot {
            connectors: vec![AppInfo {
                id: "connector_1".to_string(),
                name: "Notion".to_string(),
                description: Some("Workspace docs".to_string()),
                logo_url: None,
                logo_url_dark: None,
                icon_assets: None,
                icon_dark_assets: None,
                distribution_channel: None,
                branding: None,
                app_metadata: None,
                labels: None,
                install_url: Some("https://example.test/notion".to_string()),
                is_accessible: true,
                is_enabled: false,
                plugin_display_names: Vec::new(),
            }],
        }),
        /*is_final*/ true,
    );

    chat.add_connectors_output();
    let popup = render_bottom_popup(&chat, /*width*/ 80);
    assert!(
        popup.contains("Installed · Disabled. Press Enter to open the app page"),
        "expected selected app description to include disabled status, got:\n{popup}"
    );
    assert!(
        popup.contains("enable/disable this app."),
        "expected selected app description to mention enable/disable action, got:\n{popup}"
    );
}

#[tokio::test]
async fn apps_refresh_preserves_toggled_enabled_state() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    set_managed_auth(&mut chat);
    chat.config
        .features
        .enable(Feature::Apps)
        .expect("test config should allow feature update");
    chat.bottom_pane.set_connectors_enabled(/*enabled*/ true);

    chat.on_connectors_loaded(
        Ok(ConnectorsSnapshot {
            connectors: vec![AppInfo {
                id: "connector_1".to_string(),
                name: "Notion".to_string(),
                description: Some("Workspace docs".to_string()),
                logo_url: None,
                logo_url_dark: None,
                icon_assets: None,
                icon_dark_assets: None,
                distribution_channel: None,
                branding: None,
                app_metadata: None,
                labels: None,
                install_url: Some("https://example.test/notion".to_string()),
                is_accessible: true,
                is_enabled: true,
                plugin_display_names: Vec::new(),
            }],
        }),
        /*is_final*/ true,
    );
    chat.update_connector_enabled("connector_1", /*enabled*/ false);

    chat.on_connectors_loaded(
        Ok(ConnectorsSnapshot {
            connectors: vec![AppInfo {
                id: "connector_1".to_string(),
                name: "Notion".to_string(),
                description: Some("Workspace docs".to_string()),
                logo_url: None,
                logo_url_dark: None,
                icon_assets: None,
                icon_dark_assets: None,
                distribution_channel: None,
                branding: None,
                app_metadata: None,
                labels: None,
                install_url: Some("https://example.test/notion".to_string()),
                is_accessible: true,
                is_enabled: true,
                plugin_display_names: Vec::new(),
            }],
        }),
        /*is_final*/ true,
    );

    assert_matches!(
        &chat.connectors.cache,
        ConnectorsCacheState::Ready(snapshot)
            if snapshot
                .connectors
                .iter()
                .find(|connector| connector.id == "connector_1")
                .is_some_and(|connector| !connector.is_enabled)
    );

    chat.add_connectors_output();
    let popup = render_bottom_popup(&chat, /*width*/ 80);
    assert!(
        popup.contains("Installed · Disabled. Press Enter to open the app page"),
        "expected disabled status to persist after reload, got:\n{popup}"
    );
}

#[tokio::test]
async fn apps_popup_for_not_installed_app_uses_install_only_selected_description() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    set_managed_auth(&mut chat);
    chat.config
        .features
        .enable(Feature::Apps)
        .expect("test config should allow feature update");
    chat.bottom_pane.set_connectors_enabled(/*enabled*/ true);

    chat.on_connectors_loaded(
        Ok(ConnectorsSnapshot {
            connectors: vec![AppInfo {
                id: "connector_2".to_string(),
                name: "Linear".to_string(),
                description: Some("Project tracking".to_string()),
                logo_url: None,
                logo_url_dark: None,
                icon_assets: None,
                icon_dark_assets: None,
                distribution_channel: None,
                branding: None,
                app_metadata: None,
                labels: None,
                install_url: Some("https://example.test/linear".to_string()),
                is_accessible: false,
                is_enabled: true,
                plugin_display_names: Vec::new(),
            }],
        }),
        /*is_final*/ true,
    );

    chat.add_connectors_output();
    let popup = render_bottom_popup(&chat, /*width*/ 80);
    assert!(
        popup.contains("Can be installed. Press Enter to open the app page to install"),
        "expected selected app description to be install-only for not-installed apps, got:\n{popup}"
    );
    assert!(
        !popup.contains("enable/disable this app."),
        "did not expect enable/disable text for not-installed apps, got:\n{popup}"
    );
}



fn apply_model_list_response(chat: &mut ChatWidget, presets: Vec<ModelPreset>) {
    let request_id = chat.model_popup_request_id.expect("pending model request");
    assert!(chat.on_models_loaded(request_id, Ok(presets)));
}

#[tokio::test]
async fn model_picker_refresh_rejects_obsolete_and_unusable_replies() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
    chat.thread_id = Some(ThreadId::new());
    let initial = chat.model_catalog.try_list_models().unwrap();
    let mut refreshed = initial.clone();
    refreshed[0].description = "Updated model details".to_string();
    chat.open_model_popup();
    let old_request = chat.model_popup_request_id.unwrap();
    chat.handle_key_event(KeyEvent::from(KeyCode::Esc));
    chat.open_model_popup();
    let current_request = chat.model_popup_request_id.unwrap();
    assert!(!chat.on_models_loaded(old_request, Ok(refreshed.clone())));
    assert!(chat.model_popup_request_is_current(current_request));
    // Identical visible account fields still mark an account boundary.
    chat.update_account_state(
        chat.status_account_display.clone(),
        chat.plan_type,
        chat.has_managed_account,
        chat.has_codex_backend_auth,
    );
    assert!(!chat.on_models_loaded(current_request, Ok(refreshed.clone())));
    assert_eq!(chat.model_catalog.try_list_models().unwrap(), initial);

    for result in [
        Err("unavailable".to_string()),
        Ok(Vec::new()),
        Ok(initial.clone()),
    ] {
        chat.handle_key_event(KeyEvent::from(KeyCode::Esc));
        chat.open_model_popup();
        let request_id = chat.model_popup_request_id.unwrap();
        let before = render_bottom_popup(&chat, /*width*/ 80);
        assert!(!chat.on_models_loaded(request_id, result));
        assert!(!chat.model_popup_request_is_current(request_id));
        assert_eq!(chat.model_catalog.try_list_models().unwrap(), initial);
        assert_eq!(render_bottom_popup(&chat, /*width*/ 80), before);
    }
    chat.handle_key_event(KeyEvent::from(KeyCode::Esc));
    chat.open_model_popup();
    apply_model_list_response(&mut chat, refreshed.clone());
    assert_eq!(chat.model_catalog.try_list_models().unwrap(), refreshed);
}








#[tokio::test]
async fn server_overloaded_error_does_not_switch_models() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(Some("gpt-5.2")).await;
    chat.set_model("gpt-5.2");
    while rx.try_recv().is_ok() {}
    while op_rx.try_recv().is_ok() {}

    handle_error(
        &mut chat,
        "server overloaded",
        Some(GenexErrorInfo::ServerOverloaded),
    );

    while let Ok(event) = rx.try_recv() {
        if let AppEvent::UpdateModel(model) = event {
            assert_eq!(
                model, "gpt-5.2",
                "did not expect model switch on server-overloaded error"
            );
        }
    }

    while let Ok(event) = op_rx.try_recv() {
        if let Op::OverrideTurnContext { model, .. } = event {
            assert!(
                model.is_none(),
                "did not expect OverrideTurnContext model update on server-overloaded error"
            );
        }
    }
}



#[tokio::test]
async fn model_reasoning_selection_popup_applies_custom_effort() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
    let custom_effort = ReasoningEffortConfig::Custom("future".to_string());
    chat.set_reasoning_effort(Some(ReasoningEffortConfig::XHigh));

    let mut preset = get_available_model(&chat, "gpt-5.5");
    preset
        .supported_reasoning_efforts
        .push(ReasoningEffortPreset {
            effort: custom_effort.clone(),
            description: "Maximum available reasoning".to_string(),
        });
    chat.open_reasoning_popup(preset);
    while rx.try_recv().is_ok() {}

    chat.handle_key_event(KeyEvent::from(KeyCode::Down));
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    let selected_effort_events = std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|event| match event {
            AppEvent::UpdateReasoningEffort(effort) => Some((None, effort)),
            AppEvent::PersistModelSelection { model, effort } => Some((Some(model), effort)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        selected_effort_events,
        vec![
            (None, Some(custom_effort.clone())),
            (Some("gpt-5.5".to_string()), Some(custom_effort)),
        ]
    );
}

async fn select_ultra_with_multi_agent_thread_limit(max_threads: usize) -> (bool, Vec<String>) {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
    chat.config
        .multi_agent_v2
        .max_concurrent_threads_per_session = max_threads;
    chat.set_reasoning_effort(Some(ReasoningEffortConfig::High));

    let mut preset = get_available_model(&chat, "gpt-5.5");
    preset.default_reasoning_effort = ReasoningEffortConfig::High;
    preset.supported_reasoning_efforts = vec![
        ReasoningEffortPreset {
            effort: ReasoningEffortConfig::High,
            description: "High reasoning".to_string(),
        },
        ReasoningEffortPreset {
            effort: ReasoningEffortConfig::Ultra,
            description: "Ultra reasoning".to_string(),
        },
    ];
    chat.open_reasoning_popup(preset);
    while rx.try_recv().is_ok() {}

    chat.handle_key_event(KeyEvent::from(KeyCode::Down));
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    let advanced_preset = std::iter::from_fn(|| rx.try_recv().ok()).find_map(|event| match event {
        AppEvent::OpenAdvancedReasoningPopup { model } => Some(model),
        _ => None,
    });
    chat.open_advanced_reasoning_popup(advanced_preset.expect("advanced reasoning popup"));
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    let mut selected_ultra = false;
    let mut warnings = Vec::new();
    while let Ok(event) = rx.try_recv() {
        match event {
            AppEvent::ApplyAdvancedReasoning {
                effort: ReasoningEffortConfig::Ultra,
                ..
            } => {
                selected_ultra = true;
            }
            AppEvent::InsertHistoryCell(cell) => {
                warnings.push(lines_to_single_string(&cell.display_lines(/*width*/ 80)));
            }
            _ => {}
        }
    }

    (selected_ultra, warnings)
}


#[tokio::test]
async fn ultra_reasoning_selection_skips_warning_below_threshold() {
    let below_threshold = select_ultra_with_multi_agent_thread_limit(/*max_threads*/ 7).await;

    assert_eq!(below_threshold, (true, Vec::new()));
}

#[tokio::test]
async fn max_reasoning_selection_persists_model_selection() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
    chat.set_reasoning_effort(Some(ReasoningEffortConfig::High));

    let mut preset = get_available_model(&chat, "gpt-5.5");
    preset.supported_reasoning_efforts = vec![ReasoningEffortPreset {
        effort: ReasoningEffortConfig::Max,
        description: "Maximum reasoning".to_string(),
    }];
    chat.open_advanced_reasoning_popup(preset);
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
    assert!(events.iter().any(|event| matches!(
        event,
        AppEvent::UpdateReasoningEffort(Some(ReasoningEffortConfig::Max))
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        AppEvent::PersistModelSelection {
            model,
            effort: Some(ReasoningEffortConfig::Max),
        } if model == "gpt-5.5"
    )));
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, AppEvent::ApplyAdvancedReasoning { .. }))
    );
}

async fn assert_reasoning_shortcuts_update_effort(
    key_events: [KeyEvent; 2],
    expected_effort: ReasoningEffortConfig,
) {
    for key_event in key_events {
        let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
        chat.thread_id = Some(ThreadId::new());
        chat.set_reasoning_effort(Some(ReasoningEffortConfig::Medium));

        chat.handle_key_event(key_event);

        let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
        assert!(
            events
                .iter()
                .all(|event| !matches!(event, AppEvent::UpdateModel(_))),
            "did not expect model update event for {key_event:?}; events: {events:?}"
        );
        assert!(
            events.iter().any(|event| matches!(
                event,
                AppEvent::UpdateReasoningEffort(Some(effort)) if effort == &expected_effort
            )),
            "expected reasoning update event for {key_event:?}; events: {events:?}"
        );
        assert!(
            events
                .iter()
                .all(|event| !matches!(event, AppEvent::PersistModelSelection { .. })),
            "expected no model persistence event for {key_event:?}; events: {events:?}"
        );
    }
}

#[tokio::test]
async fn reasoning_up_shortcuts_raise_reasoning_effort() {
    assert_reasoning_shortcuts_update_effort(
        [
            KeyEvent::new(KeyCode::Char('.'), KeyModifiers::ALT),
            KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT),
        ],
        ReasoningEffortConfig::High,
    )
    .await;
}

#[tokio::test]
async fn reasoning_down_shortcuts_lower_reasoning_effort() {
    assert_reasoning_shortcuts_update_effort(
        [
            KeyEvent::new(KeyCode::Char(','), KeyModifiers::ALT),
            KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT),
        ],
        ReasoningEffortConfig::Low,
    )
    .await;
}

#[tokio::test]
async fn reasoning_shortcut_clears_armed_quit_shortcut() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
    chat.thread_id = Some(ThreadId::new());
    chat.set_reasoning_effort(Some(ReasoningEffortConfig::Medium));
    chat.arm_quit_shortcut(key_hint::ctrl(KeyCode::Char('c')));

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::ALT));

    assert!(!chat.bottom_pane.quit_shortcut_hint_visible());
    assert!(chat.quit_shortcut_expires_at.is_none());
    assert!(chat.quit_shortcut_key.is_none());
    let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, AppEvent::Exit(_))),
        "did not expect reasoning shortcut to quit; events: {events:?}"
    );
}

#[tokio::test]
async fn reasoning_shortcut_is_ignored_with_model_popup_open() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
    chat.thread_id = Some(ThreadId::new());
    chat.set_reasoning_effort(Some(ReasoningEffortConfig::Medium));
    chat.open_model_popup();

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::ALT));

    let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, AppEvent::UpdateReasoningEffort(_))),
        "did not expect reasoning update while popup is active; events: {events:?}"
    );
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, AppEvent::PersistModelSelection { .. })),
        "did not expect model persistence while popup is active; events: {events:?}"
    );
}

#[tokio::test]
async fn reasoning_up_shortcut_does_not_silently_enter_advanced_effort() {
    for (model, model_path) in [
        ("gpt-5.5", "All models → gpt-5.5"),
        ("codex-auto-test", "codex-auto-test"),
    ] {
        let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
        chat.thread_id = Some(ThreadId::new());
        let mut preset = get_available_model(&chat, "gpt-5.5");
        preset.id = model.to_string();
        preset.model = model.to_string();
        preset.display_name = model.to_string();
        preset.supported_reasoning_efforts.extend([
            ReasoningEffortPreset {
                effort: ReasoningEffortConfig::Max,
                description: "Maximum reasoning".to_string(),
            },
            ReasoningEffortPreset {
                effort: ReasoningEffortConfig::Ultra,
                description: "Ultra reasoning".to_string(),
            },
        ]);
        chat.model_catalog = std::sync::Arc::new(ModelCatalog::new(vec![preset]));
        chat.set_model(model);

        for effort in [ReasoningEffortConfig::XHigh, ReasoningEffortConfig::Max] {
            chat.set_reasoning_effort(Some(effort));
            chat.handle_key_event(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::ALT));

            let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
            assert!(events.iter().all(|event| !matches!(
                event,
                AppEvent::UpdateReasoningEffort(_) | AppEvent::ApplyAdvancedReasoning { .. }
            )));
            let messages = events
                .into_iter()
                .filter_map(|event| match event {
                    AppEvent::InsertHistoryCell(cell) => {
                        Some(lines_to_single_string(&cell.display_lines(/*width*/ 140)))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                messages,
                vec![format!(
                    "• Max and Ultra are available under /model → {model_path} → More reasoning…\n"
                )]
            );
        }
    }
}

#[tokio::test]
async fn reasoning_down_shortcut_can_leave_advanced_effort() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
    chat.thread_id = Some(ThreadId::new());
    let mut preset = get_available_model(&chat, "gpt-5.5");
    preset.supported_reasoning_efforts.extend([
        ReasoningEffortPreset {
            effort: ReasoningEffortConfig::Ultra,
            description: "Ultra reasoning".to_string(),
        },
        ReasoningEffortPreset {
            effort: ReasoningEffortConfig::Max,
            description: "Maximum reasoning".to_string(),
        },
    ]);
    chat.model_catalog = std::sync::Arc::new(ModelCatalog::new(vec![preset]));

    for (current, expected) in [
        (ReasoningEffortConfig::Ultra, ReasoningEffortConfig::Max),
        (ReasoningEffortConfig::Max, ReasoningEffortConfig::XHigh),
    ] {
        chat.set_reasoning_effort(Some(current));
        chat.handle_key_event(KeyEvent::new(KeyCode::Char(','), KeyModifiers::ALT));

        let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::UpdateReasoningEffort(Some(effort)) if effort == &expected
        )));
        assert!(
            events
                .iter()
                .all(|event| !matches!(event, AppEvent::PersistModelSelection { .. }))
        );
    }
}

#[tokio::test]
async fn reasoning_popup_shows_extra_high_with_space() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;

    set_managed_auth(&mut chat);

    let preset = get_available_model(&chat, "gpt-5.5");
    chat.open_reasoning_popup(preset);

    let popup = render_bottom_popup(&chat, /*width*/ 120);
    assert!(
        popup.contains("Extra high"),
        "expected popup to include 'Extra high'; popup: {popup}"
    );
    assert!(
        !popup.contains("Extrahigh"),
        "expected popup not to include 'Extrahigh'; popup: {popup}"
    );
}

#[tokio::test]
async fn single_reasoning_option_skips_selection() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    let single_effort = vec![ReasoningEffortPreset {
        effort: ReasoningEffortConfig::High,
        description: "Greater reasoning depth for complex or ambiguous problems".to_string(),
    }];
    let preset = ModelPreset {
        id: "model-with-single-reasoning".to_string(),
        model: "model-with-single-reasoning".to_string(),
        display_name: "model-with-single-reasoning".to_string(),
        description: "".to_string(),
        model_specialty: None,
        default_reasoning_effort: ReasoningEffortConfig::High,
        supported_reasoning_efforts: single_effort,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        default_service_tier: None,
        available_access_programs: None,
        is_default: false,
        upgrade: None,
        show_in_picker: true,
        multi_agent_version: None,
        availability_nux: None,
        supported_in_api: true,
        input_modalities: default_input_modalities(),
    };
    chat.open_reasoning_popup(preset);

    let popup = render_bottom_popup(&chat, /*width*/ 80);
    assert!(
        !popup.contains("Select Reasoning Level"),
        "expected reasoning selection popup to be skipped"
    );

    let mut events = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        events.push(ev);
    }

    assert!(
        events
            .iter()
            .any(|ev| matches!(ev, AppEvent::UpdateReasoningEffort(Some(effort)) if *effort == ReasoningEffortConfig::High)),
        "expected reasoning effort to be applied automatically; events: {events:?}"
    );
}


#[tokio::test]
async fn auto_model_advertising_advanced_effort_opens_reasoning_picker() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let mut preset = get_available_model(&chat, "gpt-5.6-terra");
    preset.id = "codex-auto-test".to_string();
    preset.model = "codex-auto-test".to_string();
    preset.display_name = "codex-auto-test".to_string();
    preset.default_reasoning_effort = ReasoningEffortConfig::Medium;
    preset
        .supported_reasoning_efforts
        .push(ReasoningEffortPreset {
            effort: ReasoningEffortConfig::Ultra,
            description: "Ultra reasoning".to_string(),
        });
    chat.open_model_popup_with_presets(vec![preset]);

    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
    assert!(events.iter().all(|event| !matches!(
        event,
        AppEvent::UpdateReasoningEffort(_)
            | AppEvent::ApplyAdvancedReasoning { .. }
            | AppEvent::PersistModelSelection { .. }
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AppEvent::OpenReasoningPopup { .. }))
    );
}

#[tokio::test]
async fn reasoning_popup_escape_returns_to_model_popup() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
    chat.thread_id = Some(ThreadId::new());
    chat.open_model_popup();

    let preset = get_available_model(&chat, "gpt-5.5");
    chat.open_reasoning_popup(preset);

    let before_escape = render_bottom_popup(&chat, /*width*/ 80);
    assert!(before_escape.contains("Select Reasoning Level"));

    chat.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    let after_escape = render_bottom_popup(&chat, /*width*/ 80);
    assert!(after_escape.contains("Select Model"));
    assert!(!after_escape.contains("Select Reasoning Level"));
}
