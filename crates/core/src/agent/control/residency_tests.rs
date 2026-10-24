use crate::StartThreadOptions;
use crate::ThreadManager;
use crate::agent::LocalAgentControl;
use crate::config::Config;
use crate::config::test_config;
use crate::ruxex_thread::RuxexThread;
use crate::thread_manager::ThreadManagerState;
use pretty_assertions::assert_eq;
use ruxex_features::Feature;
use ruxex_model_provider::ChatRequest;
use ruxex_model_provider::ListModelsRequest;
use ruxex_model_provider::ModelProvider;
use ruxex_model_provider::ProviderAccountState;
use ruxex_model_provider::ProviderAuthState;
use ruxex_model_provider::ProviderCapabilities;
use ruxex_model_provider::ProviderChatStream;
use ruxex_model_provider::ProviderConnectionKey;
use ruxex_model_provider::ProviderCredentialSnapshot;
use ruxex_model_provider::ProviderDescriptor;
use ruxex_model_provider::ProviderError;
use ruxex_model_provider::ProviderFuture;
use ruxex_model_provider::ProviderModelCatalog;
use ruxex_model_provider::ProviderRequestAuth;
use ruxex_model_provider::ProviderRequestSnapshot;
use ruxex_model_provider::ProviderResult;
use ruxex_model_provider::ProviderUsage;
use ruxex_model_provider::SharedModelProvider;
use ruxex_model_provider::UsageRequest;
use ruxex_protocol::ThreadId;
use ruxex_protocol::account::ProviderAuthStatus;
use ruxex_protocol::error::RuxexErrorDetails;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::SessionSource;
use ruxex_protocol::protocol::SubAgentSource;
use ruxex_protocol::protocol::ThreadSource;
use ruxex_protocol::protocol::TurnAbortReason;
use ruxex_protocol::protocol::TurnAbortedEvent;
use ruxex_protocol::protocol::TurnCompleteEvent;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug)]
struct TestCredentialSnapshot;

impl ProviderCredentialSnapshot for TestCredentialSnapshot {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct TestProvider;

impl ModelProvider for TestProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor::new("core-residency-test", "1")
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }

    fn chat(
        &self,
        _request: ChatRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderChatStream>> {
        Box::pin(async { Err(ProviderError::Unsupported) })
    }

    fn list_models(
        &self,
        _request: ListModelsRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderModelCatalog>> {
        Box::pin(async { Err(ProviderError::Unsupported) })
    }

    fn auth_state(&self) -> ProviderAuthState {
        ProviderAuthState {
            status: ProviderAuthStatus::Unauthenticated,
            account_id: None,
            display_name: None,
            managed_auth: false,
            revision: None,
        }
    }

    fn request_snapshot(&self) -> ProviderFuture<'_, ProviderResult<ProviderRequestSnapshot>> {
        Box::pin(async {
            Ok(ProviderRequestSnapshot {
                auth: ProviderRequestAuth {
                    authenticated: false,
                    managed_auth: false,
                    auth_header_attached: false,
                    auth_header_name: None,
                    revision: None,
                    owner_generation: None,
                },
                connection_key: ProviderConnectionKey {
                    provider_kind: "core-residency-test".to_string(),
                    auth_revision: None,
                },
                credential_snapshot: Arc::new(TestCredentialSnapshot),
            })
        })
    }

    fn account_state(&self) -> ProviderResult<ProviderAccountState> {
        Ok(ProviderAccountState {
            provider_id: "core-residency-test".to_string(),
            account: None,
            account_id: None,
            display_name: None,
            auth_status: ProviderAuthStatus::Unauthenticated,
            usage: None,
        })
    }

    fn login(
        &self,
        _credentials: BTreeMap<String, String>,
    ) -> ProviderFuture<'_, ProviderResult<()>> {
        Box::pin(async { Ok(()) })
    }

    fn logout(&self) -> ProviderFuture<'_, ProviderResult<()>> {
        Box::pin(async { Ok(()) })
    }

    fn delete(&self) -> ProviderFuture<'_, ProviderResult<bool>> {
        Box::pin(async { Ok(false) })
    }

    fn usage(&self, _request: UsageRequest) -> ProviderFuture<'_, ProviderResult<ProviderUsage>> {
        Box::pin(async { Err(ProviderError::Unsupported) })
    }
}

fn test_provider() -> SharedModelProvider {
    Arc::new(TestProvider)
}

#[tokio::test]
async fn residency_slot_reservation_unloads_oldest_idle_v2_agent() {
    let mut config = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    config.multi_agent_v2.max_concurrent_threads_per_session = 2;
    let temp_home = tempfile::tempdir().expect("create temp home");
    config.ruxex_home = temp_home.path().to_path_buf().try_into().unwrap();
    config.cwd = temp_home.path().to_path_buf().try_into().unwrap();
    let manager = ThreadManager::with_provider_and_home_for_tests(
        test_provider(),
        config.ruxex_home.to_path_buf(),
        Arc::new(ruxex_exec_server::EnvironmentManager::default_for_tests()),
    );
    let root = manager
        .start_thread(StartThreadOptions::new(config.clone()))
        .await
        .expect("start root thread");
    let control = manager.agent_control();
    let state = control.upgrade().expect("thread manager should be live");

    let first_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("first resident slot");
    let first =
        spawn_v2_subagent(&control, &state, config.clone(), root.thread_id, "worker-1").await;
    first_slot.commit(first.thread_id);
    mark_thread_completed(first.thread.as_ref()).await;

    let second_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("second resident slot should evict the first idle agent");
    match manager.get_thread(first.thread_id).await {
        Err(err) => match err.details() {
            RuxexErrorDetails::ThreadNotFound(thread_id) => assert_eq!(*thread_id, first.thread_id),
            _ => panic!("expected evicted thread to be missing, got {err:?}"),
        },
        Ok(_) => panic!("expected evicted thread to be missing"),
    }
    let second = spawn_v2_subagent(&control, &state, config, root.thread_id, "worker-2").await;
    second_slot.commit(second.thread_id);

    assert!(manager.get_thread(root.thread_id).await.is_ok());
    assert!(manager.get_thread(second.thread_id).await.is_ok());
}

#[tokio::test]
async fn interrupted_v2_agent_is_lost_after_residency_eviction() {
    let mut config = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    config.multi_agent_v2.max_concurrent_threads_per_session = 2;
    let temp_home = tempfile::tempdir().expect("create temp home");
    config.ruxex_home = temp_home.path().to_path_buf().try_into().unwrap();
    config.cwd = temp_home.path().to_path_buf().try_into().unwrap();
    let manager = ThreadManager::with_provider_and_home_for_tests(
        test_provider(),
        config.ruxex_home.to_path_buf(),
        Arc::new(ruxex_exec_server::EnvironmentManager::default_for_tests()),
    );
    let root = manager
        .start_thread(StartThreadOptions::new(config.clone()))
        .await
        .expect("start root thread");
    let control = manager.agent_control();
    let state = control.upgrade().expect("thread manager should be live");

    let first_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("first resident slot");
    let first =
        spawn_v2_subagent(&control, &state, config.clone(), root.thread_id, "worker-1").await;
    first_slot.commit(first.thread_id);
    mark_thread_interrupted(first.thread.as_ref()).await;

    let second_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("second resident slot should evict the first interrupted idle agent");
    match manager.get_thread(first.thread_id).await {
        Err(err) => match err.details() {
            RuxexErrorDetails::ThreadNotFound(thread_id) => assert_eq!(*thread_id, first.thread_id),
            _ => panic!("expected evicted thread to be missing, got {err:?}"),
        },
        Ok(_) => panic!("expected evicted thread to be missing"),
    }
    let second =
        spawn_v2_subagent(&control, &state, config.clone(), root.thread_id, "worker-2").await;
    second_slot.commit(second.thread_id);
    mark_thread_completed(second.thread.as_ref()).await;

    let err = control
        .ensure_v2_agent_loaded(config, first.thread_id, /*parent*/ None)
        .await
        .expect_err("evicted interrupted agent should stay lost");
    match err.details() {
        RuxexErrorDetails::ThreadNotFound(thread_id) => assert_eq!(*thread_id, first.thread_id),
        _ => panic!("expected ThreadNotFound, got {err:?}"),
    }

    assert!(manager.get_thread(root.thread_id).await.is_ok());
    assert!(manager.get_thread(second.thread_id).await.is_ok());
    match manager.get_thread(first.thread_id).await {
        Err(err) => match err.details() {
            RuxexErrorDetails::ThreadNotFound(thread_id) => assert_eq!(*thread_id, first.thread_id),
            _ => panic!("expected evicted thread to be missing, got {err:?}"),
        },
        Ok(_) => panic!("expected evicted thread to be missing"),
    }
}

async fn spawn_v2_subagent(
    control: &LocalAgentControl,
    state: &Arc<ThreadManagerState>,
    config: Config,
    parent_thread_id: ThreadId,
    label: &str,
) -> crate::thread_manager::NewThread {
    state
        .spawn_new_thread_with_source(
            config,
            control.clone(),
            SessionSource::SubAgent(SubAgentSource::Other(label.to_string())),
            /*history_mode*/ None,
            Some(parent_thread_id),
            /*forked_from_thread_id*/ None,
            Some(ThreadSource::Subagent),
            /*metrics_service_name*/ None,
            /*inherited_environments*/ None,
            /*inherited_exec_policy*/ None,
            /*environments*/ None,
        )
        .await
        .expect("spawn v2 subagent")
}

async fn mark_thread_completed(thread: &RuxexThread) {
    let turn = thread.session.new_default_turn().await;
    thread
        .session
        .send_event(
            turn.as_ref(),
            EventMsg::TurnComplete(TurnCompleteEvent {
                turn_id: turn.sub_id.clone(),
                started_at: None,
                last_agent_message: Some("done".to_string()),
                error: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        )
        .await;
    clear_active_turn(thread).await;
}

async fn mark_thread_interrupted(thread: &RuxexThread) {
    let turn = thread.session.new_default_turn().await;
    thread
        .session
        .send_event(
            turn.as_ref(),
            EventMsg::TurnAborted(TurnAbortedEvent {
                turn_id: Some(turn.sub_id.clone()),
                started_at: None,
                reason: TurnAbortReason::Interrupted,
                completed_at: None,
                duration_ms: None,
            }),
        )
        .await;
    clear_active_turn(thread).await;
}

async fn clear_active_turn(thread: &RuxexThread) {
    // The fixture has no task runner to clear the turn after the terminal event.
    *thread.session.active_turn.lock().await = None;
}
