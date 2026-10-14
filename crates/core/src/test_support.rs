//! Test-only helpers exposed for cross-crate integration tests.
//!
//! Production code should not depend on this module.
//! We prefer this to using a crate feature to avoid building multiple
//! permutations of the crate.

use std::path::PathBuf;
use std::sync::Arc;

use ruxex_extension_api::LoadInstructionsFuture;
use ruxex_extension_api::LoadedUserInstructions;
use ruxex_extension_api::UserInstructionsProvider;
use ruxex_http_client::HttpClientFactory;
use ruxex_http_client::OutboundProxyPolicy;
use ruxex_login::CredentialManager;
use ruxex_login::RuxexAuth;
use ruxex_model_provider::ProviderModelCatalogSource;
use ruxex_model_provider_info::ModelProviderInfo;
use ruxex_models_manager::collaboration_mode_presets;
use ruxex_models_manager::manager::SharedModelsManager;
use ruxex_models_manager::manager::build_models_manager;
use ruxex_models_manager::test_support::construct_model_info_offline_for_tests;
use ruxex_models_manager::test_support::test_models_response;
use ruxex_protocol::ThreadId;
use ruxex_protocol::config_types::CollaborationModeMask;
use ruxex_protocol::mcp::ClientMcpExtensions;
use ruxex_protocol::model_types::ModelInfo;
use ruxex_protocol::model_types::ModelPreset;
use ruxex_protocol::protocol::SessionSource;
use ruxex_providers::create_model_provider;
use once_cell::sync::Lazy;

use crate::ThreadManager;
use crate::config::Config;
use crate::responses_metadata::RuxexResponsesMetadata;
use crate::responses_metadata::RuxexResponsesRequestKind;
use crate::responses_metadata::subagent_metadata_kind;
use crate::thread_manager;
use crate::unified_exec;

static TEST_MODEL_PRESETS: Lazy<Vec<ModelPreset>> = Lazy::new(|| {
    let mut response = test_models_response()
        .unwrap_or_else(|err| panic!("test model catalog should parse: {err}"));
    response.models.sort_by_key(|model| model.priority);
    let mut presets: Vec<ModelPreset> = response.models.into_iter().map(Into::into).collect();
    ModelPreset::mark_default_by_picker_visibility(&mut presets);
    presets
});

/// Test-only provider that supplies no user instructions.
#[derive(Debug, Default)]
pub struct EmptyUserInstructionsProvider;

impl UserInstructionsProvider for EmptyUserInstructionsProvider {
    fn load_user_instructions(&self) -> LoadInstructionsFuture<'_> {
        Box::pin(async { LoadedUserInstructions::default() })
    }
}

pub fn set_thread_manager_test_mode(enabled: bool) {
    thread_manager::set_thread_manager_test_mode_for_tests(enabled);
}

pub fn set_deterministic_process_ids(enabled: bool) {
    unified_exec::set_deterministic_process_ids_for_tests(enabled);
}

pub fn credential_manager_from_auth(auth: RuxexAuth) -> Arc<CredentialManager> {
    CredentialManager::from_auth_for_testing(auth)
}

pub fn credential_manager_from_auth_with_home(
    auth: RuxexAuth,
    ruxex_home: PathBuf,
) -> Arc<CredentialManager> {
    CredentialManager::from_auth_for_testing_with_home(auth, ruxex_home)
}

pub fn with_code_mode_host_program(
    thread_manager: ThreadManager,
    host_program: PathBuf,
    config: &crate::config::Config,
) -> ThreadManager {
    thread_manager.with_code_mode_host_program_for_tests(host_program, config)
}

pub fn thread_manager_with_models_provider(
    auth: RuxexAuth,
    provider: ModelProviderInfo,
) -> ThreadManager {
    ThreadManager::with_models_provider_for_tests(auth, provider)
}

pub async fn start_thread_with_user_shell_override(
    thread_manager: &ThreadManager,
    config: Config,
    user_shell_override: crate::shell::Shell,
    _supports_provider_form_elicitation: bool,
) -> ruxex_protocol::error::Result<crate::NewThread> {
    thread_manager
        .start_thread_with_user_shell_override_for_tests(
            config,
            user_shell_override,
            ClientMcpExtensions::new(None),
        )
        .await
}

pub async fn resume_thread_from_rollout_with_user_shell_override(
    thread_manager: &ThreadManager,
    config: Config,
    rollout_path: PathBuf,
    user_shell_override: crate::shell::Shell,
    _supports_provider_form_elicitation: bool,
) -> ruxex_protocol::error::Result<crate::NewThread> {
    thread_manager
        .resume_thread_from_rollout_with_user_shell_override_for_tests(
            config,
            rollout_path,
            user_shell_override,
            ClientMcpExtensions::new(None),
        )
        .await
}

pub fn models_manager_with_provider(
    ruxex_home: PathBuf,
    credential_manager: Arc<CredentialManager>,
    provider: ModelProviderInfo,
) -> SharedModelsManager {
    let provider = create_model_provider(provider, Some(credential_manager.clone()));
    build_models_manager(
        ruxex_home,
        Arc::new(ProviderModelCatalogSource::new(provider.clone())),
        /*config_model_catalog*/ None,
    )
}

pub fn default_http_client_factory() -> HttpClientFactory {
    HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault)
}

pub fn construct_model_info_offline(model: &str, config: &Config) -> ModelInfo {
    construct_model_info_offline_for_tests(model, &config.to_models_manager_config())
}

#[derive(Clone, Copy)]
pub enum TestRuxexResponsesRequestKind {
    Turn,
    Prewarm,
    WebsocketConnection,
}

#[allow(clippy::too_many_arguments)]
pub fn responses_metadata(
    installation_id: &str,
    session_id: &str,
    thread_id: &str,
    turn_id: Option<&str>,
    window_id: String,
    session_source: &SessionSource,
    parent_thread_id: Option<ThreadId>,
    request_kind: TestRuxexResponsesRequestKind,
) -> RuxexResponsesMetadata {
    let request_kind = match request_kind {
        TestRuxexResponsesRequestKind::Turn => Some(RuxexResponsesRequestKind::Turn),
        TestRuxexResponsesRequestKind::Prewarm => Some(RuxexResponsesRequestKind::Prewarm),
        TestRuxexResponsesRequestKind::WebsocketConnection => None,
    };
    RuxexResponsesMetadata {
        turn_id: request_kind.and(turn_id.map(ToString::to_string)),
        request_kind,
        parent_thread_id,
        subagent_kind: request_kind.and_then(|_| subagent_metadata_kind(session_source)),
        ..RuxexResponsesMetadata::new(
            installation_id.to_string(),
            session_id.to_string(),
            thread_id.to_string(),
            window_id,
        )
    }
}

pub fn with_parent_turn(mut metadata: RuxexResponsesMetadata, id: &str) -> RuxexResponsesMetadata {
    metadata.parent_turn_id = Some(id.to_string());
    metadata
}

pub fn all_model_presets() -> &'static Vec<ModelPreset> {
    &TEST_MODEL_PRESETS
}

pub fn builtin_collaboration_mode_presets() -> Vec<CollaborationModeMask> {
    collaboration_mode_presets::builtin_collaboration_mode_presets()
}
