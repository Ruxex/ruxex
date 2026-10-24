//! Configured inputs retained independently of future thread settings.

use crate::config::Constrained;
use crate::config::ConstraintResult;
use ruxex_config::ConfigRequirements;
use ruxex_models_manager::ModelsManagerConfig;
use ruxex_models_manager::manager::ModelsManager;
use ruxex_otel::SessionTelemetry;
use ruxex_protocol::config_types::CollaborationMode;
use ruxex_protocol::config_types::ReasoningSummary;
use ruxex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE;
use ruxex_protocol::config_types::ServiceTier;
use ruxex_protocol::model_types::ModelInfo;
use ruxex_protocol::model_types::ReasoningEffort;
use ruxex_protocol::protocol::AskForApproval;
use std::sync::Arc;

/// Model and execution settings selected for an individual model step within
/// a turn. A turn may contain several steps, each using its own captured settings.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StepSettings {
    pub(crate) collaboration_mode: CollaborationMode,
    /// `None` follows the pinned model's default.
    pub(crate) reasoning_summary: Option<ReasoningSummary>,
    /// Normalized requested tier. Startup retains initial-model filtering;
    /// resolution filters the effective tier against the pinned model.
    pub(crate) service_tier: Option<String>,
    pub(crate) approval_policy: Constrained<AskForApproval>,
}

/// Immutable configured choices, pinned model metadata, and effective request values.
///
/// One snapshot may produce several fresh `StepContext`s. Replacing the
/// snapshot does not change steps or actions that have already captured it.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedStepSettings {
    /// Inputs for later sparse patches. Unset defaults and unsupported requested
    /// tiers must not be reconstructed from the effective values below.
    selected: Arc<StepSettings>,
    pub(crate) model_info: Arc<ModelInfo>,
    /// Effective request summary: the configured value or the pinned model's default.
    pub(crate) reasoning_summary: ReasoningSummary,
    /// Tier sent on requests, filtered by model support.
    pub(crate) service_tier: Option<String>,
}

impl ResolvedStepSettings {
    /// Derives request values from selected settings and pinned metadata.
    pub(super) fn new(selected: Arc<StepSettings>, model_info: Arc<ModelInfo>) -> Self {
        let reasoning_summary = selected
            .reasoning_summary
            .unwrap_or(model_info.default_reasoning_summary);
        let service_tier = super::get_service_tier(selected.service_tier.clone(), &model_info);
        Self {
            selected,
            model_info,
            reasoning_summary,
            service_tier,
        }
    }

    pub(crate) fn reasoning_effort(&self) -> Option<&ReasoningEffort> {
        self.selected
            .collaboration_mode
            .settings
            .reasoning_effort
            .as_ref()
    }

    pub(crate) fn effective_reasoning_effort(&self) -> Option<ReasoningEffort> {
        self.reasoning_effort()
            .or(self.model_info.default_reasoning_level.as_ref())
            .cloned()
    }

    pub(crate) fn effective_collaboration_mode(&self) -> CollaborationMode {
        let mut collaboration_mode = self.selected.collaboration_mode.clone();
        collaboration_mode.settings.model = self.model_info.slug.clone();
        collaboration_mode
    }

    pub(crate) fn approval_policy(&self) -> AskForApproval {
        self.selected.approval_policy.value()
    }

    /// Retained inputs for constructing a snapshot against different model metadata.
    pub(super) fn selected(&self) -> &StepSettings {
        &self.selected
    }

    pub(super) fn selected_collaboration_mode(&self) -> &CollaborationMode {
        &self.selected.collaboration_mode
    }

    /// Applies sparse edits to the retained selection, preserving pinned metadata
    /// unless the model selection changes, then resolves request values.
    pub(super) async fn apply_update(
        &self,
        update: &StepSettingsUpdate,
        constraints: &StepSettingsConstraints<'_>,
        models_manager: &dyn ModelsManager,
        overrides: &ModelInfoOverrides,
    ) -> ConstraintResult<Self> {
        let selected = self.selected.apply(update, constraints)?;
        let model_info =
            if selected.collaboration_mode.model() == self.selected.collaboration_mode.model() {
                Arc::clone(&self.model_info)
            } else {
                Arc::new(selected.resolve_model_info(models_manager, overrides).await)
            };
        let next = Self::new(Arc::new(selected), model_info);
        Ok(next)
    }

    /// Rechecks the retained selection against current managed constraints.
    pub(super) fn revalidate(
        &self,
        constraints: &StepSettingsConstraints<'_>,
    ) -> ConstraintResult<()> {
        // TODO: Revisit the split with StepSettings::validate, which does not
        // check inherited approval values against the supplied managed allow-lists.
        // Sharing these checks would also make ordinary settings updates stricter
        // after a requirements refresh.
        self.selected.validate(constraints)?;
        constraints
            .requirements
            .approval_policy
            .can_set(&self.approval_policy())
    }

    pub(super) fn telemetry(&self, base: &SessionTelemetry) -> SessionTelemetry {
        base.clone().with_model(
            self.selected.collaboration_mode.model(),
            &self.model_info.slug,
        )
    }
}

/// Explicit startup overrides applied to catalog-derived model metadata.
/// Construct from `Config::to_models_manager_config()` so model-derived base
/// instructions are not mistaken for explicit overrides.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModelInfoOverrides {
    pub(crate) context_window: Option<i64>,
    pub(crate) auto_compact_token_limit: Option<i64>,
    pub(crate) tool_output_token_limit: Option<usize>,
    pub(crate) base_instructions: Option<String>,
}

impl From<ModelsManagerConfig> for ModelInfoOverrides {
    fn from(config: ModelsManagerConfig) -> Self {
        Self {
            context_window: config.model_context_window,
            auto_compact_token_limit: config.model_auto_compact_token_limit,
            tool_output_token_limit: config.tool_output_token_limit,
            base_instructions: config.base_instructions,
        }
    }
}

impl ModelInfoOverrides {
    pub(crate) fn models_manager_config(&self) -> ModelsManagerConfig {
        ModelsManagerConfig {
            model_context_window: self.context_window,
            model_auto_compact_token_limit: self.auto_compact_token_limit,
            tool_output_token_limit: self.tool_output_token_limit,
            base_instructions: self.base_instructions.clone(),
            // The models manager already owns its catalog.
            model_catalog: None,
        }
    }
}

/// Sparse edits applied independently to each settings owner.
///
/// Do not materialize a partial update against one settings owner and reuse
/// that full value for another. Merge the requested edits with each target.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StepSettingsUpdate {
    pub(crate) model: Option<String>,
    pub(crate) effort: Option<Option<ReasoningEffort>>,
    /// A complete collaboration mode takes precedence over model and effort edits.
    pub(crate) collaboration_mode: Option<CollaborationMode>,
    pub(crate) reasoning_summary: Option<ReasoningSummary>,
    pub(crate) service_tier: Option<Option<String>>,
    pub(crate) approval_policy: Option<AskForApproval>,
}

/// Constraints used when applying and validating a candidate settings version.
/// Future settings use the proposed environment; active settings use the
/// environment already admitted for that execution.
pub(crate) struct StepSettingsConstraints<'a> {
    pub(crate) requirements: &'a ConfigRequirements,
}

impl StepSettings {
    /// Resolves the selected model using the session's explicit startup overrides.
    pub(super) async fn resolve_model_info(
        &self,
        models_manager: &dyn ModelsManager,
        overrides: &ModelInfoOverrides,
    ) -> ModelInfo {
        let config = overrides.models_manager_config();
        models_manager
            .get_model_info(self.collaboration_mode.model(), &config)
            .await
    }

    /// Applies edits and validates the result against the supplied constraints.
    /// Callers must supply the constraints of the proposed target environment.
    pub(crate) fn apply(
        &self,
        update: &StepSettingsUpdate,
        constraints: &StepSettingsConstraints<'_>,
    ) -> ConstraintResult<Self> {
        let mut next = self.clone();
        next.collaboration_mode = update.collaboration_mode.clone().unwrap_or_else(|| {
            self.collaboration_mode.with_updates(
                update.model.clone(),
                update.effort.clone(),
                /*developer_instructions*/ None,
            )
        });
        if let Some(summary) = update.reasoning_summary {
            next.reasoning_summary = Some(summary);
        }
        if let Some(service_tier) = update.service_tier.clone() {
            // TODO(aibrahim): Remove once v2 clients no longer send the legacy
            // An unknown service tier value.
            next.service_tier = Some(match service_tier {
                Some(service_tier) => ServiceTier::from_request_value(&service_tier)
                    .map_or(service_tier, |service_tier| {
                        service_tier.request_value().to_string()
                    }),
                None => SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string(),
            });
        }
        if let Some(approval_policy) = update.approval_policy {
            next.approval_policy.set(approval_policy)?;
        }
        next.validate(constraints)?;
        Ok(next)
    }

    /// Checks directly constructed settings or rechecks changed constraints.
    pub(super) fn validate(
        &self,
        _constraints: &StepSettingsConstraints<'_>,
    ) -> ConstraintResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruxex_models_manager::model_info::model_info_from_slug;
    use ruxex_protocol::config_types::ModeKind;

    fn selected_settings() -> StepSettings {
        StepSettings {
            collaboration_mode: CollaborationMode {
                mode: ModeKind::Default,
                settings: ruxex_protocol::config_types::Settings {
                    model: "initial-model".to_string(),
                    reasoning_effort: None,
                    developer_instructions: Some("developer".to_string()),
                },
            },
            reasoning_summary: None,
            service_tier: None,
            approval_policy: Constrained::allow_any(AskForApproval::OnRequest),
        }
    }

    #[test]
    fn applies_sparse_settings_and_resolves_effective_values() {
        let selected = selected_settings();
        let requirements = ConfigRequirements::default();
        let constraints = StepSettingsConstraints {
            requirements: &requirements,
        };
        let update = StepSettingsUpdate {
            effort: Some(Some(ReasoningEffort::High)),
            reasoning_summary: Some(ReasoningSummary::Detailed),
            service_tier: Some(Some("priority".to_string())),
            approval_policy: Some(AskForApproval::Never),
            ..Default::default()
        };
        let updated = selected.apply(&update, &constraints).expect("valid update");
        assert_eq!(updated.collaboration_mode.model(), "initial-model");
        assert_eq!(
            updated.collaboration_mode.reasoning_effort(),
            Some(ReasoningEffort::High)
        );
        assert_eq!(updated.service_tier.as_deref(), Some("priority"));
        assert_eq!(updated.approval_policy.value(), AskForApproval::Never);

        let resolved = ResolvedStepSettings::new(
            Arc::new(updated),
            Arc::new(model_info_from_slug("resolved-model")),
        );
        assert_eq!(
            resolved.effective_collaboration_mode().model(),
            "resolved-model"
        );
        assert_eq!(
            resolved.effective_reasoning_effort(),
            Some(ReasoningEffort::High)
        );
        assert_eq!(resolved.approval_policy(), AskForApproval::Never);
        assert!(
            resolved
                .selected_collaboration_mode()
                .settings
                .developer_instructions
                .is_some()
        );
    }

    #[test]
    fn model_info_overrides_round_trip_through_manager_config() {
        let overrides = ModelInfoOverrides {
            context_window: Some(1000),
            auto_compact_token_limit: Some(800),
            tool_output_token_limit: Some(256),
            base_instructions: Some("base".to_string()),
        };
        let config = overrides.models_manager_config();
        assert_eq!(config.model_context_window, Some(1000));
        assert_eq!(config.model_auto_compact_token_limit, Some(800));
        assert_eq!(config.tool_output_token_limit, Some(256));
        assert_eq!(config.base_instructions.as_deref(), Some("base"));
        assert!(config.model_catalog.is_none());
        assert_eq!(ModelInfoOverrides::from(config), overrides);
    }
}
