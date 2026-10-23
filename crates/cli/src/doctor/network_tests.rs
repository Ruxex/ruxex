use pretty_assertions::assert_eq;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use ruxex_core::config::ConfigBuilder;
use ruxex_model_provider::ProviderDescriptor;

use super::super::CheckStatus;
use super::super::ProviderAuthReachabilityMode;
use super::super::mount_provider_home_for_doctor;
use super::super::provider_reachability_check;
use super::super::provider_reachability_plan;
use super::super::provider_reachability_plan_from_parts;

#[tokio::test]
async fn provider_reachability_uses_chat_completions_for_chat_providers() {
    let home = tempfile::tempdir().expect("temporary RUXEX_HOME");
    let mut config = ConfigBuilder::default()
        .ruxex_home(home.path().to_path_buf())
        .build()
        .await
        .expect("config should build");
    config.model_provider.name = "Chat provider".to_string();
    config.model_provider.base_url = Some("https://provider.example/v1".to_string());
    config.model_provider.wire_api = serde_json::from_str("\"chat\"")
        .expect("chat should be a valid wire API");

    let plan = provider_reachability_plan(
        &config,
        &ProviderDescriptor::new("responses", "1"),
    );

    assert_eq!(
        plan.endpoints[0].url,
        "https://provider.example/v1/chat/completions"
    );
}

#[tokio::test]
async fn doctor_mounts_the_injected_provider_home() {
    let home = tempfile::tempdir().expect("temporary RUXEX_HOME");
    let config = ConfigBuilder::default()
        .ruxex_home(home.path().to_path_buf())
        .build()
        .await
        .expect("config should build");

    let marker = mount_provider_home_for_doctor(
        &config,
        &ProviderDescriptor::new("custom-provider", "1"),
    )
    .expect("provider home should mount");

    assert_eq!(marker.provider_kind, "custom-provider");
}

#[tokio::test]
async fn provider_reachability_does_not_probe_provider_owned_http() {
    let home = tempfile::tempdir().expect("temporary RUXEX_HOME");
    let config = ConfigBuilder::default()
        .ruxex_home(home.path().to_path_buf())
        .build()
        .await
        .expect("config should build");

    let plan = provider_reachability_plan(
        &config,
        &ProviderDescriptor::new("custom-provider", "1"),
    );

    assert!(plan.endpoints.is_empty());
}

#[tokio::test]
async fn provider_reachability_rejects_proxy_authentication_challenges() {
    let proxy = MockServer::start().await;
    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(407))
        .mount(&proxy)
        .await;
    let provider_base_url = format!("{}/backend-api/", proxy.uri());
    let plan = provider_reachability_plan_from_parts(
        ProviderAuthReachabilityMode::ApiKey,
        "provider",
        "provider",
        Some(&provider_base_url),
        /*provider_query_params*/ None,
        "responses",
    );

    let check = provider_reachability_check(plan).await;
    assert_eq!(check.status, CheckStatus::Fail);
    assert!(
        check
            .details
            .join(" ")
            .contains("proxy authentication required (HTTP 407)")
    );
}
