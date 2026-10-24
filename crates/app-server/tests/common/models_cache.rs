use ruxex_core::test_support::all_model_presets;
use ruxex_protocol::config_types::ReasoningSummary;
use ruxex_protocol::model_types::ConfigShellToolType;
use ruxex_protocol::model_types::ModelInfo;
use ruxex_protocol::model_types::ModelMessages;
use ruxex_protocol::model_types::ModelPreset;
use ruxex_protocol::model_types::ModelVisibility;
use ruxex_protocol::model_types::TruncationPolicyConfig;
use ruxex_protocol::model_types::default_input_modalities;
use std::path::Path;

/// Convert a ModelPreset to ModelInfo for cache storage.
fn preset_to_info(preset: &ModelPreset, priority: i32) -> ModelInfo {
    ModelInfo {
        slug: preset.id.clone(),
        display_name: preset.display_name.clone(),
        description: Some(preset.description.clone()),
        default_reasoning_level: Some(preset.default_reasoning_effort.clone()),
        supported_reasoning_levels: preset.supported_reasoning_efforts.clone(),
        shell_type: ConfigShellToolType::UnifiedExec,
        visibility: if preset.show_in_picker {
            ModelVisibility::List
        } else {
            ModelVisibility::Hide
        },
        supported_in_api: preset.supported_in_api,
        priority,
        additional_speed_tiers: preset.additional_speed_tiers.clone(),
        service_tiers: preset.service_tiers.clone(),
        default_service_tier: preset.default_service_tier.clone(),
        upgrade: preset.upgrade.as_ref().map(Into::into),
        model_messages: Some(ModelMessages {
            persistent_instructions: None,
            tools: None,
            instructions_template: Some("base instructions".to_string()),
            approvals: None,
            collaboration_modes: None,
            permissions: None,
            multi_agent: None,
            token_budget: None,
            confirmation_policies: None,
        }),
        include_skills_usage_instructions: false,
        include_plugin_usage_instructions: false,
        supports_reasoning_summary_parameter: true,
        default_reasoning_summary: ReasoningSummary::Auto,
        support_verbosity: false,
        default_verbosity: None,
        availability_nux: preset.availability_nux.clone(),
        apply_patch_tool_type: None,
        web_search_tool_type: Default::default(),
        truncation_policy: TruncationPolicyConfig::bytes(/*limit*/ 10_000),
        supports_image_detail_original: false,
        context_window: Some(272_000),
        max_context_window: None,
        auto_compact_token_limit: None,
        comp_hash: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
        input_modalities: default_input_modalities(),
        used_fallback_model_metadata: false,
        supports_search_tool: false,
        supports_experimental_context: false,
        use_responses_lite: false,
        node_repl_disabled: false,
        model_specialty: None,
        tool_mode: None,
        multi_agent_version: preset.multi_agent_version,
        multi_agent_reasoning_effort: None,
    }
}

/// Write a models_cache.json file to the ruxex home directory.
/// This prevents ModelsManager from making network requests to refresh models.
/// The cache will be treated as fresh (within TTL) and used instead of fetching from the network.
/// Uses bundled-catalog-derived presets, converted to ModelInfo format.
pub async fn write_models_cache(ruxex_home: &Path) -> std::io::Result<()> {
    // Get a stable bundled-catalog-derived preset list and filter for picker-visible entries.
    let presets: Vec<&ModelPreset> = all_model_presets()
        .iter()
        .filter(|preset| preset.show_in_picker)
        .collect();
    // Convert presets to ModelInfo, assigning priorities (lower = earlier in list).
    // Priority is used for sorting, so the first model gets the lowest priority.
    let models: Vec<ModelInfo> = presets
        .iter()
        .enumerate()
        .map(|(idx, preset)| {
            // Lower priority = earlier in list.
            let priority = idx as i32;
            preset_to_info(preset, priority)
        })
        .collect();

    write_models_cache_with_models(ruxex_home, models).await
}

/// Write a models_cache.json file with specific models.
/// Useful when tests need specific models to be available.
pub async fn write_models_cache_with_models(
    ruxex_home: &Path,
    models: Vec<ModelInfo>,
) -> std::io::Result<()> {
    let config = ruxex_core::config::ConfigBuilder::default()
        .loader_overrides(ruxex_config::LoaderOverrides::without_managed_config_for_tests())
        .ruxex_home(ruxex_home.to_path_buf())
        .build()
        .await?;
    let auth = ruxex_login::RuxexAuth::from_auth_storage(
        ruxex_home,
        config.cli_auth_credentials_store_mode,
        Some(&config.managed_auth_base_url),
        config.auth_keyring_backend_kind(),
        &ruxex_login::test_support::transport_default_auth_route_config(),
    )
    .await?;
    let cache = ruxex_model_provider::test_support::models_cache_entry(
        &config.model_provider,
        auth.as_ref(),
        models,
    );
    let cache_path = ruxex_home.join("models_cache.json");
    std::fs::write(cache_path, serde_json::to_string_pretty(&cache)?)
}
