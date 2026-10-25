//! Verifies that turn-abort lifecycle callbacks finish before terminal events.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_response_once;
use core_test_support::responses::sse;
use core_test_support::responses::sse_response;
use core_test_support::responses::start_mock_server;
use core_test_support::test_ruxex::test_ruxex;
use core_test_support::wait_for_event;
use ruxex_extension_api::ExtensionFuture;
use ruxex_extension_api::ExtensionRegistryBuilder;
use ruxex_extension_api::TurnAbortInput;
use ruxex_extension_api::TurnLifecycleContributor;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::Op;
use ruxex_protocol::protocol::TurnAbortReason;
use ruxex_protocol::user_input::UserInput;
use tokio::sync::Notify;
use tokio::time::timeout;

#[derive(Default)]
struct AbortGate {
    entered: Notify,
    release: Notify,
    completed: Mutex<Vec<(String, TurnAbortReason)>>,
}

impl TurnLifecycleContributor for AbortGate {
    fn on_turn_abort<'a>(&'a self, input: TurnAbortInput<'a>) -> ExtensionFuture<'a, ()> {
        Box::pin(async move {
            self.entered.notify_one();
            self.release.notified().await;
            self.completed
                .lock()
                .expect("abort callback records")
                .push((input.turn_store.level_id().to_string(), input.reason));
        })
    }
}

#[tokio::test]
async fn abort_lifecycle_finishes_before_turn_aborted() -> Result<()> {
    let server = start_mock_server().await;
    mount_response_once(
        &server,
        sse_response(sse(vec![
            ev_response_created("response"),
            ev_completed("response"),
        ]))
        .set_delay(Duration::from_secs(60)),
    )
    .await;

    let gate = Arc::new(AbortGate::default());
    let mut extensions = ExtensionRegistryBuilder::new();
    extensions.turn_lifecycle_contributor(gate.clone());
    let test = test_ruxex()
        .with_extensions(Arc::new(extensions.build()))
        .build_with_auto_env(&server)
        .await?;
    test.ruxex
        .start_or_steer_turn(ruxex_core::TurnInputRequest::user_input(vec![
            UserInput::Text {
                text: "interrupt me".to_string(),
                text_elements: Vec::new(),
            },
        ]))
        .await?;
    let turn_started = wait_for_event(&test.ruxex, |event| {
        matches!(event, EventMsg::TurnStarted(_))
    })
    .await;
    let EventMsg::TurnStarted(turn_started) = turn_started else {
        unreachable!();
    };

    test.ruxex.submit(Op::Interrupt).await?;
    timeout(Duration::from_secs(5), gate.entered.notified()).await?;

    let early_event = timeout(
        Duration::from_millis(100),
        wait_for_event(&test.ruxex, |event| {
            matches!(event, EventMsg::TurnAborted(_))
        }),
    )
    .await;
    assert!(
        early_event.is_err(),
        "TurnAborted arrived before the abort callback finished"
    );

    gate.release.notify_one();
    let EventMsg::TurnAborted(aborted) = wait_for_event(&test.ruxex, |event| {
        matches!(event, EventMsg::TurnAborted(_))
    })
    .await
    else {
        unreachable!();
    };
    assert_eq!(aborted.turn_id, Some(turn_started.turn_id.clone()));
    assert_eq!(aborted.reason, TurnAbortReason::Interrupted);
    assert_eq!(
        *gate.completed.lock().expect("abort callback records"),
        vec![(turn_started.turn_id, TurnAbortReason::Interrupted)],
    );

    Ok(())
}
