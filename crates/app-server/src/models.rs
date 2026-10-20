use std::sync::Arc;

use ruxex_app_server_protocol::Model;
use ruxex_app_server_protocol::ModelServiceTier;
use ruxex_app_server_protocol::ModelUpgradeInfo;
use ruxex_app_server_protocol::ReasoningEffortOption;
use ruxex_core::ThreadManager;
use ruxex_http_client::HttpClientFactory;
use ruxex_models_manager::manager::RefreshStrategy;
use ruxex_protocol::error::Result as CoreResult;
use ruxex_protocol::model_types::ModelPreset;
use ruxex_protocol::model_types::ReasoningEffortPreset;

pub async fn supported_models(
    thread_manager: Arc<ThreadManager>,
    include_hidden: bool,
    http_client_factory: HttpClientFactory,
) -> CoreResult<Vec<Model>> {
    Ok(thread_manager
        .list_models_result(RefreshStrategy::OnlineIfUncached, http_client_factory)
        .await?
        .into_iter()
        .filter(|preset| include_hidden || preset.show_in_picker)
        .map(model_from_preset)
        .collect())
}

fn model_from_preset(preset: ModelPreset) -> Model {
    Model {
        id: preset.id.to_string(),
        model: preset.model.to_string(),
        upgrade: preset.upgrade.as_ref().map(|upgrade| upgrade.id.clone()),
        upgrade_info: preset.upgrade.as_ref().map(|upgrade| ModelUpgradeInfo {
            model: upgrade.id.clone(),
            upgrade_copy: upgrade.upgrade_copy.clone(),
            model_link: upgrade.model_link.clone(),
            migration_markdown: upgrade.migration_markdown.clone(),
            retirement_at: upgrade
                .retirement_at
                .as_ref()
                .map(chrono::DateTime::timestamp),
        }),
        availability_nux: preset.availability_nux.map(Into::into),
        display_name: preset.display_name.to_string(),
        description: preset.description.to_string(),
        model_specialty: preset.model_specialty,
        hidden: !preset.show_in_picker,
        supported_reasoning_efforts: reasoning_efforts_from_preset(
            preset.supported_reasoning_efforts,
        ),
        default_reasoning_effort: preset.default_reasoning_effort,
        input_modalities: preset.input_modalities,
        multi_agent_version: preset.multi_agent_version.map(Into::into),
        additional_speed_tiers: preset.additional_speed_tiers,
        service_tiers: preset
            .service_tiers
            .into_iter()
            .map(|service_tier| ModelServiceTier {
                id: service_tier.id,
                name: service_tier.name,
                description: service_tier.description,
            })
            .collect(),
        default_service_tier: preset.default_service_tier,
        available_access_programs: preset.available_access_programs.map(Into::into),
        is_default: preset.is_default,
    }
}

fn reasoning_efforts_from_preset(
    efforts: Vec<ReasoningEffortPreset>,
) -> Vec<ReasoningEffortOption> {
    efforts
        .into_iter()
        .map(|preset| ReasoningEffortOption {
            reasoning_effort: preset.effort,
            description: preset.description,
        })
        .collect()
}
