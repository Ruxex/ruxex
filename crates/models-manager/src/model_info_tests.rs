use super::*;
use crate::ModelsManagerConfig;
use pretty_assertions::assert_eq;
use ruxex_prompts::render_model_instructions;
use ruxex_protocol::model_types::ApprovalMessages;
use ruxex_protocol::model_types::CollaborationModeMessages;
use ruxex_protocol::model_types::ConfirmationPolicies;
use ruxex_protocol::model_types::ModelTokenBudgetConfig;
use ruxex_protocol::model_types::MultiAgentMessages;
use ruxex_protocol::model_types::MultiAgentModeMessages;
use ruxex_protocol::model_types::MultiAgentRoleMessages;
use ruxex_protocol::model_types::MultiAgentToolMessages;
use ruxex_protocol::model_types::PermissionMessages;
use ruxex_protocol::model_types::ToolMessage;
use ruxex_protocol::model_types::ToolMessages;

#[test]
fn base_instruction_override_is_literal_and_preserves_catalog_messages() {
    let override_instructions = "override {{ literal }}\n# General\nKeep me";
    let persistent_instructions = "Follow up on the active task.";
    let async_message_description = "Catalog async message description.";
    let mut model = model_info_from_slug("unknown-model");
    let approvals = ApprovalMessages {
        on_request: Some("user approvals".to_string()),
        never: Some("never approvals".to_string()),
        unless_trusted: Some("unless-trusted approvals".to_string()),
    };
    let collaboration_modes = CollaborationModeMessages {
        default: Some("default instructions".to_string()),
        plan: Some("plan instructions".to_string()),
    };
    let permissions = PermissionMessages {
        danger_full_access: Some("danger".to_string()),
        workspace_write: Some(String::new()),
        read_only: None,
    };
    let multi_agent = MultiAgentMessages {
        role: Some(MultiAgentRoleMessages {
            root: Some("root base".to_string()),
            subagent: Some("subagent base".to_string()),
        }),
        mode: Some(MultiAgentModeMessages {
            explicit: Some("explicit mode".to_string()),
            proactive: Some("proactive mode".to_string()),
            hint_text: Some("mode hint".to_string()),
        }),
    };
    let token_budget = ModelTokenBudgetConfig {
        enabled: false,
        use_history_notes_extension: false,
        reminder_threshold_tokens: 128,
        reminder_message_template: "budget reminder".to_string(),
        guidance_message: "budget guidance".to_string(),
        auto_compact_fallback_prompt: "compact prompt".to_string(),
        auto_compact_fallback_buffer_tokens: 64,
    };
    let confirmation_policies = ConfirmationPolicies {
        browser_use: Some("# Browser policy\n\n{{literal_markdown}}\n".to_string()),
        computer_use: Some("  # Native policy\r\n\n${native_markdown}\n".to_string()),
    };
    let mut messages = ModelMessages {
        persistent_instructions: Some(persistent_instructions.to_string()),
        tools: Some(ToolMessages {
            send_user_message_async: Some(ToolMessage {
                description: Some(async_message_description.to_string()),
            }),
            multi_agent: Some(MultiAgentToolMessages {
                spawn_agent: Some(ToolMessage {
                    description: Some("Catalog spawn description.".to_string()),
                }),
            }),
        }),
        instructions_template: Some("template".to_string()),
        approvals: Some(approvals),
        collaboration_modes: Some(collaboration_modes),
        permissions: Some(permissions),
        multi_agent: Some(multi_agent),
        token_budget: Some(token_budget),
        confirmation_policies: Some(confirmation_policies),
    };
    model.model_messages = Some(messages.clone());
    let config = ModelsManagerConfig {
        base_instructions: Some(override_instructions.to_string()),
        ..Default::default()
    };

    let updated = with_config_overrides(model, &config);

    messages.instructions_template = Some(override_instructions.to_string());
    assert_eq!(updated.model_messages, Some(messages));
    assert_eq!(render_model_instructions(&updated), override_instructions);
}

#[test]
fn catalog_instruction_is_preserved_without_personality_processing() {
    let instructions = "Intro\n# General\nKeep me";
    let mut model = model_info_from_slug("unknown-model");
    model
        .model_messages
        .as_mut()
        .expect("fallback model messages")
        .instructions_template = Some(instructions.to_string());

    assert_eq!(
        render_model_instructions(&with_config_overrides(
            model,
            &ModelsManagerConfig::default()
        )),
        instructions
    );
}

#[test]
fn explicit_empty_base_instructions_stay_empty() {
    let mut model = model_info_from_slug("unknown-model");
    model
        .model_messages
        .as_mut()
        .expect("fallback model messages")
        .instructions_template = Some("Intro\n# General\nKeep me".to_string());
    let config = ModelsManagerConfig {
        base_instructions: Some(String::new()),
        ..Default::default()
    };

    let updated = with_config_overrides(model, &config);

    assert_eq!(
        updated
            .model_messages
            .as_ref()
            .and_then(|messages| messages.instructions_template.as_deref()),
        Some("")
    );
    assert_eq!(render_model_instructions(&updated), "");
}

#[test]
fn unknown_model_uses_builtin_instruction_template() {
    let model = model_info_from_slug("unknown-model");

    assert_eq!(render_model_instructions(&model), BASE_INSTRUCTIONS);
    assert!(!BASE_INSTRUCTIONS.contains("# Personality"));
    assert!(model.used_fallback_model_metadata);
}

#[test]
fn model_context_window_override_clamps_to_max_context_window() {
    let mut model = model_info_from_slug("unknown-model");
    model.context_window = Some(273_000);
    model.max_context_window = Some(400_000);
    let config = ModelsManagerConfig {
        model_context_window: Some(500_000),
        ..Default::default()
    };

    let updated = with_config_overrides(model.clone(), &config);
    let mut expected = model;
    expected.context_window = Some(400_000);

    assert_eq!(updated, expected);
}

#[test]
fn model_context_window_uses_model_value_without_override() {
    let mut model = model_info_from_slug("unknown-model");
    model.context_window = Some(273_000);
    model.max_context_window = Some(400_000);
    let config = ModelsManagerConfig::default();

    let updated = with_config_overrides(model.clone(), &config);

    assert_eq!(updated, model);
}
