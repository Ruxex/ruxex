use genex_core::TurnInputRequest;
use genex_login::AuthHeaders;
use genex_login::GenexAuth;
use genex_model_provider_info::ModelProviderInfo;
use genex_model_provider_info::WireApi;
use genex_model_provider_info::create_responses_provider_with_base_url;
use genex_protocol::protocol::AuthRecoveryEvent;
use genex_protocol::protocol::EventMsg;
use genex_protocol::user_input::UserInput;
use genex_utils_redacted_string::RedactedString;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_response_sequence;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_genex::test_genex;
use core_test_support::wait_for_event;
use http::HeaderMap;
use http::HeaderValue;
use http::header::AUTHORIZATION;
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;
use std::path::Path;
use std::path::PathBuf;
use tokio::process::Command;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn header_auth_is_attached_to_responses_requests() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response_mock = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer external"));
    headers.insert("x-external-auth", HeaderValue::from_static("enabled"));
    let mut builder = test_genex().with_auth(GenexAuth::Headers(
        AuthHeaders::new(headers).with_account_id("account-123"),
    ));
    let test = builder.build_with_auto_env(&server).await?;

    test.submit_turn("hello").await?;

    let request = response_mock.single_request();
    assert_eq!(
        request.header("authorization").as_deref(),
        Some("Bearer external")
    );
    assert_eq!(
        request.header("x-external-auth").as_deref(),
        Some("enabled")
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn custom_provider_does_not_receive_ambient_auth_headers() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response_mock = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer ambient"));
    let provider =
        create_responses_provider_with_base_url(&format!("{}/v1", server.uri()), WireApi::Responses);
    let mut builder = test_genex()
        .with_auth(GenexAuth::Headers(AuthHeaders::new(headers)))
        .with_config(move |config| {
            config.model_provider_id = provider.name.clone();
            config.model_provider = provider;
        });
    let test = builder.build_with_auto_env(&server).await?;

    test.submit_turn("hello").await?;

    let request = response_mock.single_request();
    assert_eq!(request.header("authorization"), None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn custom_provider_uses_explicit_bearer_without_ambient_account() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response_mock = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer ambient"));
    let mut provider =
        create_responses_provider_with_base_url(&format!("{}/v1", server.uri()), WireApi::Responses);
    provider.experimental_bearer_token = Some("provider-token".into());
    let mut builder = test_genex()
        .with_auth(GenexAuth::Headers(AuthHeaders::new(headers)))
        .with_config(move |config| {
            config.model_provider_id = provider.name.clone();
            config.model_provider = provider;
        });
    let test = builder.build_with_auto_env(&server).await?;

    test.submit_turn("hello").await?;

    let request = response_mock.single_request();
    assert_eq!(
        request.header("authorization").as_deref(),
        Some("Bearer provider-token")
    );
    Ok(())
}
