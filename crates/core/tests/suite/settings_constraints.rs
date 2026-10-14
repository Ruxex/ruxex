use anyhow::Result;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::test_ruxex::test_ruxex;
use core_test_support::wait_for_event;
use ruxex_config::test_support::CloudConfigBundleFixture;
use ruxex_core::RuxexThread;
use ruxex_core::TurnInputRequest;
use ruxex_core::TurnInputSubmission;
use ruxex_core::config::Constrained;
use ruxex_features::Feature;
use ruxex_models_manager::model_info::model_info_from_slug;
use ruxex_protocol::config_types::ApprovalsReviewer;
use ruxex_protocol::model_types::ApprovalMessages;
use ruxex_protocol::model_types::ModelCatalog;
use ruxex_protocol::model_types::PermissionMessages;
use ruxex_protocol::models::PermissionProfile;
use ruxex_protocol::protocol::AskForApproval;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::RuxexErrorInfo;
use ruxex_protocol::protocol::Op;
use ruxex_protocol::protocol::ThreadSettingsOverrides;
use ruxex_protocol::protocol::ThreadSettingsSnapshot;
use ruxex_protocol::user_input::UserInput;
use pretty_assertions::assert_eq;
use std::time::Duration;
use test_case::test_case;
use tokio::time::timeout;

const INITIAL_MODEL: &str = "legacy-model";
const PROTECTED_MODEL: &str = "example-model-2";
const REVIEW_INSTRUCTIONS: &str = "Protected-model automatic review instructions.";
const PERMISSION_INSTRUCTIONS: &str = "Protected-model read-only instructions.";

#[derive(Clone, Copy)]
enum SettingsOperation {
    Standalone,
    TurnStart,
}

impl SettingsOperation {
    async fn submit(
        self,
        ruxex: &RuxexThread,
        thread_settings: ThreadSettingsOverrides,
    ) -> Result<ThreadSettingsSnapshot> {
        let id = match self {
            Self::Standalone => ruxex.submit(Op::ThreadSettings { thread_settings }).await?,
            Self::TurnStart => {
                let result = ruxex
                    .start_or_steer_turn(
                        TurnInputRequest::user_input(vec![UserInput::Text {
                            text: "use the protected model".to_string(),
                            text_elements: Vec::new(),
                        }])
                        .with_thread_settings(thread_settings),
                    )
                    .await?;
                let TurnInputSubmission::Started { turn_id } = result else {
                    anyhow::bail!("expected a new turn, got {result:?}");
                };
                turn_id
            }
        };
        timeout(Duration::from_secs(10), async {
            loop {
                let event = ruxex.next_event().await?;
                if event.id != id {
                    continue;
                }
                match event.msg {
                    EventMsg::ThreadSettingsApplied(applied) => return Ok(applied.thread_settings),
                    EventMsg::Error(error) => {
                        assert_eq!(error.ruxex_error_info, Some(RuxexErrorInfo::BadRequest));
                        anyhow::bail!(error.message);
                    }
                    _ => {}
                }
            }
        })
        .await?
    }
}

#[test_case(SettingsOperation::Standalone; "standalone settings")]
#[test_case(SettingsOperation::TurnStart; "turn-start settings")]
#[tokio::test]
async fn protected_model_settings_use_the_proposed_permissions(
    operation: SettingsOperation,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let response = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("response"),
            responses::ev_completed("response"),
        ]),
    )
    .await;
    let mut protected = model_info_from_slug(PROTECTED_MODEL);
    let messages = protected.model_messages.as_mut().expect("model messages");
    messages.approvals = Some(ApprovalMessages {
        on_request: None,
        on_request_auto_review: Some(REVIEW_INSTRUCTIONS.to_string()),
        never: None,
        unless_trusted: None,
    });
    messages.permissions = Some(PermissionMessages {
        danger_full_access: None,
        workspace_write: None,
        read_only: Some(PERMISSION_INSTRUCTIONS.to_string()),
    });
    let test = test_ruxex()
        .with_model(INITIAL_MODEL)
        .with_cloud_config_bundle(
            CloudConfigBundleFixture::loader_with_enterprise_requirement(
                "[auto_review]\nrequired_on_models = [\"example-model-2\"]\n",
            ),
        )
        .with_config(move |config| {
            config.model_catalog = Some(ModelCatalog {
                models: vec![model_info_from_slug(INITIAL_MODEL), protected],
            });
            config.permissions.approval_policy = Constrained::allow_any(AskForApproval::OnRequest);
            config.approvals_reviewer = ApprovalsReviewer::User;
            config
                .permissions
                .set_permission_profile(PermissionProfile::Disabled)
                .expect("full-access profile should be allowed");
            config
                .features
                .enable(Feature::GuardianApproval)
                .expect("test config should allow Guardian");
        })
        .build_with_auto_env(&server)
        .await?;
    let initial = test.ruxex.thread_settings_snapshot().await;
    assert_eq!(initial.permission_profile, PermissionProfile::Disabled);
    let model_update = ThreadSettingsOverrides {
        model: Some(PROTECTED_MODEL.to_string()),
        ..Default::default()
    };

    let error = operation
        .submit(&test.ruxex, model_update.clone())
        .await
        .expect_err("the protected model requires restricted permissions");
    assert!(
        error.to_string().contains("you need to use auto review"),
        "{error}"
    );
    assert_eq!(test.ruxex.thread_settings_snapshot().await, initial);
    assert!(response.requests().is_empty());

    let expected = ThreadSettingsSnapshot {
        model: PROTECTED_MODEL.to_string(),
        collaboration_mode: initial.collaboration_mode.with_updates(
            Some(PROTECTED_MODEL.to_string()),
            /*effort*/ None,
            /*developer_instructions*/ None,
        ),
        approvals_reviewer: ApprovalsReviewer::AutoReview,
        permission_profile: PermissionProfile::read_only(),
        active_permission_profile: None,
        ..initial
    };
    let applied = operation
        .submit(
            &test.ruxex,
            ThreadSettingsOverrides {
                permission_profile: Some(PermissionProfile::read_only()),
                ..model_update
            },
        )
        .await?;
    assert_eq!(applied, expected);
    assert_eq!(test.ruxex.thread_settings_snapshot().await, expected);
    if let SettingsOperation::TurnStart = operation {
        wait_for_event(&test.ruxex, |event| {
            matches!(event, EventMsg::TurnComplete(_))
        })
        .await;
    }

    // Even an update with no step-settings edits must revalidate the existing
    // protected model against the proposed permissions.
    let error = operation
        .submit(
            &test.ruxex,
            ThreadSettingsOverrides {
                permission_profile: Some(PermissionProfile::Disabled),
                ..Default::default()
            },
        )
        .await
        .expect_err("full access must invalidate the protected model settings");
    assert!(
        error.to_string().contains("you need to use auto review"),
        "{error}"
    );
    assert_eq!(test.ruxex.thread_settings_snapshot().await, expected);

    if let SettingsOperation::Standalone = operation {
        test.submit_text_turn("use the committed settings").await?;
    }
    let request = response.single_request();
    assert_eq!(request.body_json()["model"], PROTECTED_MODEL);
    let instructions = request.message_input_texts("developer").join("\n");
    assert!(instructions.contains(REVIEW_INSTRUCTIONS), "{instructions}");
    assert!(
        instructions.contains(PERMISSION_INSTRUCTIONS),
        "{instructions}"
    );
    Ok(())
}
