use std::sync::Arc;

use ruxex_exec_server::EnvironmentManager;
use ruxex_exec_server::ExecServerRuntimePaths;
use ruxex_extension_api::ExtensionRegistry;
use ruxex_extension_api::UserInstructionsProvider;
use ruxex_model_provider::SharedModelProvider;
use ruxex_protocol::error::Result as RuxexResult;
use ruxex_protocol::error::RuxexErr;
use ruxex_protocol::models::ResponseItem;
use ruxex_protocol::protocol::SessionSource;
use ruxex_protocol::user_input::UserInput;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::resolve_installation_id;
use crate::session::session::Session;
use crate::session::turn::build_prompt;
use crate::state_db_bridge::StateDbHandle;
use crate::thread_manager::StartThreadOptions;
use crate::thread_manager::ThreadManager;
use crate::thread_manager::thread_store_from_config;

/// Build the model-visible `input` list for a single debug turn.
#[doc(hidden)]
pub async fn build_prompt_input(
    mut config: Config,
    provider: SharedModelProvider,
    input: Vec<UserInput>,
    state_db: Option<StateDbHandle>,
    extensions: Arc<ExtensionRegistry<Config>>,
    user_instructions_provider: Arc<dyn UserInstructionsProvider>,
) -> RuxexResult<Vec<ResponseItem>> {
    config.ephemeral = true;

    let local_runtime_paths = ExecServerRuntimePaths::from_optional_paths(
        config.ruxex_self_exe.clone(),
        config.ruxex_linux_sandbox_exe.clone(),
    )?;

    let thread_store = thread_store_from_config(&config, state_db.clone());
    let installation_id = resolve_installation_id(&config.ruxex_home).await?;
    let thread_manager = ThreadManager::new(
        &config,
        Arc::clone(&provider),
        crate::build_models_manager_with_provider(&config, Arc::clone(&provider)),
        SessionSource::Exec,
        Arc::new(
            EnvironmentManager::from_ruxex_home(
                config.ruxex_home.clone(),
                Some(local_runtime_paths),
                config.http_client_factory(),
            )
            .await
            .map_err(|err| RuxexErr::Fatal(err.to_string()))?,
        ),
        extensions,
        user_instructions_provider,
        crate::thread_manager::passthrough_image_store(),
        thread_store,
        crate::local_agent_graph_store_from_state_db(state_db.as_ref()),
        installation_id,
        /*attestation_provider*/ None,
        /*external_time_provider*/ None,
    );
    let thread = thread_manager
        .start_thread(StartThreadOptions::new(config))
        .await?;

    let output = build_prompt_input_from_session(&thread.thread.session, input).await;
    let shutdown = thread.thread.shutdown_and_wait().await;
    let _removed = thread_manager.remove_thread(&thread.thread_id).await;

    shutdown?;
    output
}

pub(crate) async fn build_prompt_input_from_session(
    sess: &Arc<Session>,
    input: Vec<UserInput>,
) -> RuxexResult<Vec<ResponseItem>> {
    let turn_context = sess.new_default_turn().await;
    // Prompt debugging builds a standalone request without entering run_turn.
    let step_context = sess
        .capture_step_context(Arc::clone(&turn_context), &CancellationToken::new())
        .await?;
    sess.record_context_updates_and_set_reference_context_item(step_context.as_ref())
        .await?;

    if !input.is_empty() {
        let response_item = sess.response_item_from_user_input(input);
        sess.record_conversation_items(
            turn_context.as_ref(),
            &step_context.settings.model_info,
            std::slice::from_ref(&response_item),
        )
        .await;
    }

    let prompt_input = sess
        .clone_history()
        .await
        .for_prompt(&step_context.settings.model_info.input_modalities);
    let base_instructions = sess.get_base_instructions().await;
    let prompt = build_prompt(prompt_input, step_context.as_ref(), base_instructions);

    Ok(prompt.input)
}
