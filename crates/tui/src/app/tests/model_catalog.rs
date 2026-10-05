use super::*;
use assert_matches::assert_matches;
use genex_config::types::ModelAvailabilityNuxConfig;
use genex_protocol::config_types::ServiceTier;
use genex_protocol::model_types::ModelAvailabilityNux;
use pretty_assertions::assert_eq;
use tokio::sync::mpsc::unbounded_channel;

fn all_model_presets() -> Vec<ModelPreset> {
    crate::test_support::TEST_MODEL_PRESETS.clone()
}

#[tokio::test]
async fn model_picker_refresh_updates_app_catalog_from_app_server() -> Result<()> {
    let (mut app, mut rx, _op_rx) = make_test_app_with_channels().await;
    let mut app_server = start_config_write_test_app_server(&app).await?;
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let service_tier = Some(ServiceTier::Flex.request_value());
    let thread_id = ThreadId::new();
    let mut session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    session.model = "example-model".to_string();
    app.primary_thread_id = Some(thread_id);
    app.active_thread_id = Some(thread_id);
    app.primary_session_configured = Some(session.clone());
    app.thread_event_channels.insert(
        thread_id,
        ThreadEventChannel::new_with_session(/*capacity*/ 4, session.clone(), Vec::new()),
    );
    app.chat_widget.handle_thread_session(session.clone());
    app.chat_widget.open_model_popup();
    let request_id = std::iter::from_fn(|| rx.try_recv().ok())
        .find_map(|event| match event {
            AppEvent::FetchModels { request_id } => Some(request_id),
            _ => None,
        })
        .expect("initial model request");
    let mut hidden = all_model_presets();
    for preset in &mut hidden {
        preset.show_in_picker = false;
    }
    Box::pin(app.handle_event(
        &mut tui,
        &mut app_server,
        AppEvent::ModelsLoaded {
            request_id,
            result: Ok(hidden),
        },
    ))
    .await?;
    app.chat_widget.open_model_popup();
    assert!(app.chat_widget.no_modal_or_popup_active());
    let request = std::iter::from_fn(|| rx.try_recv().ok())
        .find(|event| matches!(event, AppEvent::FetchModels { .. }))
        .expect("catalog request");

    Box::pin(app.handle_event(&mut tui, &mut app_server, request)).await?;
    let mut loaded = tokio::time::timeout(Duration::from_secs(/*secs*/ 10), async {
        loop {
            if let Some(event @ AppEvent::ModelsLoaded { .. }) = rx.recv().await {
                break event;
            }
        }
    })
    .await?;
    let AppEvent::ModelsLoaded {
        result: Ok(models), ..
    } = &mut loaded
    else {
        panic!("model/list should return models");
    };
    assert!(!models.is_empty());
    for model in models.iter_mut() {
        model.is_default = model.model == "example-model";
        if model.is_default {
            model.service_tiers = vec![genex_protocol::model_types::ModelServiceTier {
                id: ServiceTier::Flex.request_value().to_string(),
                name: "Flex".to_string(),
                description: "Flexible processing".to_string(),
            }];
            model.default_service_tier = Some(ServiceTier::Flex.request_value().to_string());
        }
    }
    let expected = models.clone();
    Box::pin(app.handle_event(&mut tui, &mut app_server, loaded)).await?;
    assert!(app.chat_widget.no_modal_or_popup_active());
    app.chat_widget.open_model_popup();
    assert!(!app.chat_widget.no_modal_or_popup_active());

    session.service_tier = service_tier.map(str::to_string);
    let channel = &app.thread_event_channels[&thread_id];
    let cached = channel.store.lock().await.session.clone();
    assert_eq!(
        (&app.primary_session_configured, &cached),
        (&Some(session.clone()), &Some(session.clone()))
    );
    app.chat_widget.handle_thread_session(test_thread_session(
        ThreadId::new(),
        test_path_buf("/tmp/other"),
    ));
    app.chat_widget.handle_thread_session(cached.unwrap());
    assert_eq!(app.chat_widget.current_service_tier(), service_tier);
    let stale = AppEvent::ModelsLoaded {
        request_id: uuid::Uuid::new_v4(),
        result: Ok(all_model_presets()),
    };
    Box::pin(app.handle_event(&mut tui, &mut app_server, stale)).await?;
    assert_eq!(
        (
            app.model_catalog.try_list_models().unwrap(),
            app.chat_widget.model_catalog().try_list_models().unwrap(),
        ),
        (expected.clone(), expected)
    );
    let mut config = app.config.clone();
    config.service_tier = None;
    for (model, expected_tier) in [(None, service_tier), (Some("provider-model-d"), Some("default"))] {
        config.model = model.map(str::to_string);
        let started = app_server.start_thread(&config).await?;
        assert_eq!(started.session.service_tier.as_deref(), expected_tier);
    }
    app_server.shutdown().await?;
    Ok(())
}

fn model_presets_with_test_upgrades() -> Vec<ModelPreset> {
    let mut presets = all_model_presets();
    let mut preset = presets
        .iter()
        .find(|preset| preset.model == "example-model")
        .expect("current model template")
        .clone();
    // Catalog-provided migration metadata is independent of the retired model's prompts.
    preset.id = "example-model-2".to_string();
    preset.model = "example-model-2".to_string();
    preset.upgrade = Some(ModelUpgrade {
        id: "example-model".to_string(),
        model_link: None,
        upgrade_copy: None,
        migration_markdown: None,
        retirement_at: None,
    });
    presets.push(preset);
    presets
}

fn model_availability_nux_config(shown_count: &[(&str, u32)]) -> ModelAvailabilityNuxConfig {
    ModelAvailabilityNuxConfig {
        shown_count: shown_count
            .iter()
            .map(|(model, count)| ((*model).to_string(), *count))
            .collect(),
    }
}

fn model_migration_copy_to_plain_text(copy: &crate::model_migration::ModelMigrationCopy) -> String {
    if let Some(markdown) = copy.markdown.as_ref() {
        return markdown.clone();
    }
    let mut s = String::new();
    for span in &copy.heading {
        s.push_str(&span.content);
    }
    s.push('\n');
    s.push('\n');
    for line in &copy.content {
        for span in &line.spans {
            s.push_str(&span.content);
        }
        s.push('\n');
    }
    s
}

#[tokio::test]
async fn model_migration_prompt_only_shows_for_deprecated_models() {
    let seen = BTreeMap::new();
    let presets = model_presets_with_test_upgrades();
    assert!(should_show_model_migration_prompt(
        "example-model-2", "example-model", &seen, &presets
    ));
    assert!(should_show_model_migration_prompt(
        "legacy-model",
        "provider-model-d",
        &seen,
        &presets
    ));
    assert!(!should_show_model_migration_prompt(
        "legacy-model", "legacy-model", &seen, &presets
    ));
}

#[test]
fn retired_model_migration_respects_catalog_metadata() {
    let mut presets = model_presets_with_test_upgrades();
    let current = presets
        .iter_mut()
        .find(|preset| preset.model == "example-model-2")
        .expect("test preset");
    current.id = "legacy-model".to_string();
    current.model = "legacy-model".to_string();
    let expected = current.upgrade.clone();
    assert_eq!(
        model_upgrade_for_migration("legacy-model", &presets),
        expected
    );

    presets
        .iter_mut()
        .find(|preset| preset.model == "legacy-model")
        .expect("catalog preset")
        .upgrade = None;
    assert_eq!(model_upgrade_for_migration("legacy-model", &presets), None);
}

#[test]
fn select_model_availability_nux_picks_only_eligible_model() {
    let mut presets = all_model_presets();
    presets.iter_mut().for_each(|preset| {
        preset.availability_nux = None;
    });
    let target = presets
        .iter_mut()
        .find(|preset| preset.model == "provider-model-d")
        .expect("target preset present");
    target.availability_nux = Some(ModelAvailabilityNux {
        message: "provider-model-d is available".to_string(),
    });

    let selected = select_model_availability_nux(&presets, &model_availability_nux_config(&[]));

    assert_eq!(
        selected,
        Some(StartupTooltipOverride {
            model_slug: "provider-model-d".to_string(),
            message: "provider-model-d is available".to_string(),
        })
    );
}

#[test]
fn select_model_availability_nux_does_not_fall_back_to_older_announcement() {
    let mut presets = all_model_presets();
    presets.iter_mut().for_each(|preset| {
        preset.availability_nux = None;
    });
    let model_5 = presets
        .iter_mut()
        .find(|preset| preset.model == "provider-model-d")
        .expect("provider-model-d preset present");
    model_5.availability_nux = Some(ModelAvailabilityNux {
        message: "provider-model-d is available".to_string(),
    });
    let model_5_2 = presets
        .iter_mut()
        .find(|preset| preset.model == "example-model")
        .expect("example-model preset present");
    model_5_2.availability_nux = Some(ModelAvailabilityNux {
        message: "example-model is available".to_string(),
    });

    let selected = select_model_availability_nux(
        &presets,
        &model_availability_nux_config(&[("provider-model-d", MODEL_AVAILABILITY_NUX_MAX_SHOW_COUNT)]),
    );

    assert_eq!(selected, None);
}

#[test]
fn select_model_availability_nux_uses_existing_model_order_as_priority() {
    let mut presets = all_model_presets();
    presets.iter_mut().for_each(|preset| {
        preset.availability_nux = None;
    });
    let first = presets
        .iter_mut()
        .find(|preset| preset.model == "example-model")
        .expect("example-model preset present");
    first.availability_nux = Some(ModelAvailabilityNux {
        message: "first".to_string(),
    });
    let second = presets
        .iter_mut()
        .find(|preset| preset.model == "provider-model-d")
        .expect("provider-model-d preset present");
    second.availability_nux = Some(ModelAvailabilityNux {
        message: "second".to_string(),
    });

    let selected = select_model_availability_nux(&presets, &model_availability_nux_config(&[]));

    assert_eq!(
        selected,
        Some(StartupTooltipOverride {
            model_slug: "provider-model-d".to_string(),
            message: "second".to_string(),
        })
    );
}

#[test]
fn select_model_availability_nux_returns_none_when_all_models_are_exhausted() {
    let mut presets = all_model_presets();
    presets.iter_mut().for_each(|preset| {
        preset.availability_nux = None;
    });
    let target = presets
        .iter_mut()
        .find(|preset| preset.model == "provider-model-d")
        .expect("target preset present");
    target.availability_nux = Some(ModelAvailabilityNux {
        message: "provider-model-d is available".to_string(),
    });

    let selected = select_model_availability_nux(
        &presets,
        &model_availability_nux_config(&[("provider-model-d", MODEL_AVAILABILITY_NUX_MAX_SHOW_COUNT)]),
    );

    assert_eq!(selected, None);
}

#[tokio::test]
async fn prepare_startup_tooltip_override_persists_model_availability_nux_count() {
    let genex_home = tempdir().expect("temp genex home");
    let config = ConfigBuilder::default()
        .genex_home(genex_home.path().to_path_buf())
        .build()
        .await
        .expect("config");
    let mut presets = all_model_presets();
    presets.iter_mut().for_each(|preset| {
        preset.availability_nux = None;
    });
    let target = presets
        .iter_mut()
        .find(|preset| preset.model == "provider-model-d")
        .expect("target preset present");
    target.availability_nux = Some(ModelAvailabilityNux {
        message: "provider-model-d is available".to_string(),
    });

    let mut local_settings = crate::local_settings::LocalSettings::from(&config);
    let tooltip = prepare_startup_tooltip_override(
        &mut local_settings,
        &presets,
        /*is_first_run*/ false,
    )
    .await;

    assert_eq!(tooltip.as_deref(), Some("provider-model-d is available"));
    assert_eq!(
        local_settings.tui.model_availability_nux.shown_count,
        HashMap::from([("provider-model-d".to_string(), 1)])
    );

    let reloaded = ConfigBuilder::default()
        .genex_home(genex_home.path().to_path_buf())
        .build()
        .await
        .expect("reloaded config");
    assert_eq!(
        reloaded.model_availability_nux.shown_count,
        HashMap::from([("provider-model-d".to_string(), 1)])
    );
}


#[tokio::test]
async fn model_migration_prompt_respects_hide_flag_and_self_target() {
    let presets = all_model_presets();
    let mut seen = BTreeMap::new();
    seen.insert("legacy-model".to_string(), "provider-model-c".to_string());
    assert!(!should_show_model_migration_prompt(
        "legacy-model",
        "provider-model-c",
        &seen,
        &presets
    ));
    assert!(!should_show_model_migration_prompt(
        "provider-model-c",
        "provider-model-c",
        &seen,
        &presets
    ));
}

#[tokio::test]
async fn model_migration_prompt_skips_when_target_missing_or_hidden() {
    let mut available = all_model_presets();
    available.retain(|preset| preset.model != "provider-model-c");

    assert!(!should_show_model_migration_prompt(
        "legacy-model",
        "provider-model-c",
        &BTreeMap::new(),
        &available,
    ));

    assert!(target_preset_for_upgrade(&available, "provider-model-c").is_none());

    let mut with_hidden_target = all_model_presets();
    let target = with_hidden_target
        .iter_mut()
        .find(|preset| preset.model == "provider-model-c")
        .expect("target preset present");
    target.show_in_picker = false;

    assert!(!should_show_model_migration_prompt(
        "legacy-model",
        "provider-model-c",
        &BTreeMap::new(),
        &with_hidden_target,
    ));
    assert!(target_preset_for_upgrade(&with_hidden_target, "provider-model-c").is_none());
}
