use genex_protocol::model_types::ModelsResponse;
use genex_protocol::model_types::WebSearchToolType;
use genex_protocol::protocol::MultiAgentVersion;

/// Applies Bedrock capability constraints to a catalog returned by the provider.
///
/// The model identities and metadata remain backend-owned. This function only removes
/// capabilities that the selected Bedrock wire endpoint cannot execute.
pub(crate) fn normalize_bedrock_catalog(mut catalog: ModelsResponse) -> ModelsResponse {
    for model in &mut catalog.models {
        model.additional_speed_tiers.clear();
        model.service_tiers.clear();
        model.default_service_tier = None;
        model.web_search_tool_type = WebSearchToolType::Text;
        model.multi_agent_version = Some(MultiAgentVersion::V1);
    }
    catalog
}

#[cfg(test)]
mod tests {
    use genex_protocol::model_types::ModelInfo;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn remote_bedrock_catalog_normalizes_unsupported_capabilities() {
        let model: ModelInfo = serde_json::from_value(serde_json::json!({
            "slug": "backend-model",
            "display_name": "Backend Model",
            "description": null,
            "default_reasoning_level": "medium",
            "supported_reasoning_levels": [],
            "shell_type": "shell_command",
            "visibility": "list",
            "supported_in_api": true,
            "priority": 0,
            "additional_speed_tiers": ["fast"],
            "service_tiers": [{
                "id": "standard",
                "name": "Standard",
                "description": "Standard"
            }],
            "default_service_tier": "standard",
            "web_search_tool_type": "text_and_image",
            "experimental_supported_tools": []
        }))
        .expect("test model should deserialize");

        let normalized = normalize_bedrock_catalog(ModelsResponse {
            models: vec![model],
        });

        let model = &normalized.models[0];
        assert_eq!(model.additional_speed_tiers, Vec::<String>::new());
        assert_eq!(model.service_tiers, Vec::new());
        assert_eq!(model.default_service_tier, None);
        assert_eq!(model.web_search_tool_type, WebSearchToolType::Text);
        assert_eq!(model.multi_agent_version, Some(MultiAgentVersion::V1));
    }
}
