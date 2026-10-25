use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use futures::FutureExt;
use pretty_assertions::assert_eq;
use ruxex_exec_server::RouteAwareHttpClient;
use ruxex_http_client::HttpClientFactory;
use ruxex_http_client::OutboundProxyPolicy;
use serde_json::Value;
use serde_json::json;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;

use super::*;
use crate::ema_auth_policy::validate_ema_oauth_endpoint;
use crate::ema_claims::ID_JAG_TOKEN_TYPE;
use crate::ema_claims::validate_oidc_identity_assertion;

fn http_client() -> Arc<dyn HttpClient> {
    Arc::new(RouteAwareHttpClient::new(HttpClientFactory::new(
        OutboundProxyPolicy::ReqwestDefault,
    )))
}

fn unique_form_fields(body: &[u8]) -> HashMap<String, String> {
    let pairs = url::form_urlencoded::parse(body)
        .into_owned()
        .collect::<Vec<_>>();
    let fields = pairs.iter().cloned().collect::<HashMap<_, _>>();
    assert_eq!(
        pairs.len(),
        fields.len(),
        "OAuth form must not contain duplicate fields"
    );
    fields
}

fn jwt(claims: &Value) -> String {
    format!(
        "{}.{}.c2lnbmF0dXJl",
        URL_SAFE_NO_PAD.encode(br#"{"alg":"ES256","typ":"oauth-id-jag+jwt"}"#),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).expect("serialize claims"))
    )
}

fn claims(issuer: &str, audience: &str, resource: &str) -> Value {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time")
        .as_secs();
    json!({"iss":issuer,"aud":audience,"sub":"user","client_id":"mcp-client",
        "jti":"unique-jag","iat":now,"exp":now + 3600,"resource":resource,"scope":"files.read"})
}

fn jag_response(claims: &Value) -> Value {
    let mut response = json!({"access_token":jwt(claims),"issued_token_type":ID_JAG_TOKEN_TYPE,
        "token_type":"N_A","resource":claims["resource"]});
    if let Some(scope) = claims.get("scope") {
        response["scope"] = scope.clone();
    }
    response
}

fn token_response() -> Value {
    json!({"access_token":"resource-token","token_type":"Bearer","expires_in":300})
}

#[tokio::test]
async fn public_client_round_trip_preserves_signed_narrowing() -> Result<()> {
    let requested_scopes = ["files.read".to_string(), "files.write".to_string()];
    let client = http_client();
    for (scopes, echo_scope, refresh_token) in [
        (requested_scopes.as_slice(), true, "opaque-refresh-token"),
        (&[], true, "opaque-refresh-token"),
        (&[], false, "opaque-refresh-token"),
        (requested_scopes.as_slice(), true, ""),
        (requested_scopes.as_slice(), true, " \t"),
    ] {
        let idp_server = MockServer::start().await;
        let resource_server = MockServer::start().await;
        let issuer = format!("{}/idp", idp_server.uri());
        let audience = format!("{}/as", resource_server.uri());
        let resource = format!("{}/mcp", resource_server.uri());
        let mut jag = jag_response(&claims(&issuer, &audience, &resource));
        if !echo_scope {
            jag.as_object_mut()
                .expect("ID-JAG response")
                .remove("scope");
        }
        let valid = !refresh_token.trim().is_empty();
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(jag.clone()))
            .expect(u64::from(valid))
            .mount(&idp_server)
            .await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_response()))
            .expect(u64::from(valid))
            .mount(&resource_server)
            .await;
        let expected_subject = refresh_token.to_string();
        let result = exchange_id_jag(EmaIdJagExchangeRequest {
            resource: &resource,
            scopes,
            mcp_client_id: "mcp-client",
            authorization_server_issuer: &audience,
            authorization_server_token_endpoint: &format!("{audience}/token"),
            idp_token_endpoint: &format!("{issuer}/token"),
            idp_issuer: &issuer,
            idp_client_id: "idp-client",
            refresh_token: refresh_token.to_string(),
            idp_http_client: Arc::clone(&client),
            resource_http_client: Arc::clone(&client),
        })
        .boxed()
        .await;
        if !valid {
            assert!(result.is_err(), "invalid subject must fail before HTTP");
            assert!(
                idp_server
                    .received_requests()
                    .await
                    .expect("requests")
                    .is_empty()
            );
            continue;
        }
        assert_eq!(
            result?,
            EmaAccessToken {
                access_token: "resource-token".to_string(),
                expires_in: Some(Duration::from_secs(300)),
            }
        );
        let idp_requests = idp_server.received_requests().await.expect("IdP requests");
        let resource_requests = resource_server
            .received_requests()
            .await
            .expect("resource requests");
        assert_eq!(idp_requests.len(), 1);
        assert_eq!(resource_requests.len(), 1);
        assert!(idp_requests[0].headers.get("authorization").is_none());
        assert!(resource_requests[0].headers.get("authorization").is_none());
        let mut idp_form = unique_form_fields(&idp_requests[0].body);
        let resource_form = unique_form_fields(&resource_requests[0].body);
        assert_eq!(
            idp_form.remove("scope"),
            (!scopes.is_empty()).then(|| scopes.join(" "))
        );
        assert_eq!(
            idp_form,
            HashMap::from([
                (
                    "grant_type".to_string(),
                    TOKEN_EXCHANGE_GRANT_TYPE.to_string()
                ),
                (
                    "requested_token_type".to_string(),
                    ID_JAG_TOKEN_TYPE.to_string(),
                ),
                ("subject_token".to_string(), expected_subject),
                (
                    "subject_token_type".to_string(),
                    "urn:ietf:params:oauth:token-type:refresh_token".to_string(),
                ),
                ("audience".to_string(), audience.clone()),
                ("resource".to_string(), resource.clone()),
                ("client_id".to_string(), "idp-client".to_string()),
            ])
        );
        assert_eq!(
            resource_form,
            HashMap::from([
                ("grant_type".to_string(), JWT_BEARER_GRANT_TYPE.to_string()),
                (
                    "assertion".to_string(),
                    jag["access_token"].as_str().expect("JAG").to_string()
                ),
                ("client_id".to_string(), "mcp-client".to_string()),
            ])
        );
    }
    Ok(())
}

#[test]
fn identity_and_credential_destinations_are_bound() {
    for endpoint in [
        "http://idp.example/token",
        "https://user:pass@idp.example/token",
        "https://idp.example/token#fragment",
    ] {
        assert!(
            validate_ema_oauth_endpoint(endpoint, "IdP").is_err(),
            "accepted {endpoint}"
        );
    }
    let original = claims("https://idp.example", "idp-client", "https://mcp.example");
    assert!(
        validate_oidc_identity_assertion(&jwt(&original), "https://idp.example", "idp-client")
            .is_ok()
    );
    for (field, value) in [
        ("iss", json!("https://other.example")),
        ("aud", json!(["idp-client", "other"])),
        ("azp", json!("other")),
        ("exp", json!(0)),
        ("sub", json!("")),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        assert!(
            validate_oidc_identity_assertion(&jwt(&changed), "https://idp.example", "idp-client")
                .is_err(),
            "accepted changed {field}"
        );
    }
}

#[tokio::test]
async fn sdk_maps_idp_invalid_grant_to_enterprise_identity() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "invalid_grant",
            "error_description": "secret-assertion-sentinel"
        })))
        .mount(&server)
        .await;
    let issuer = format!("{}/idp", server.uri());
    let audience = format!("{}/as", server.uri());
    let resource = format!("{}/mcp", server.uri());
    let error = exchange_id_jag(EmaIdJagExchangeRequest {
        resource: &resource,
        scopes: &[],
        mcp_client_id: "mcp-client",
        authorization_server_issuer: &audience,
        authorization_server_token_endpoint: &format!("{audience}/token"),
        idp_token_endpoint: &format!("{issuer}/token"),
        idp_issuer: &issuer,
        idp_client_id: "idp-client",
        refresh_token: "secret-refresh-token".to_string(),
        idp_http_client: http_client(),
        resource_http_client: http_client(),
    })
    .await
    .expect_err("invalid grant should fail");
    assert_eq!(
        error.downcast_ref::<EmaAuthFailure>(),
        Some(&EmaAuthFailure::InvalidGrant {
            grant_source: EmaInvalidGrantSource::EnterpriseIdentity,
        })
    );
    assert!(!format!("{error:#}").contains("secret-assertion-sentinel"));
}

#[tokio::test]
async fn sdk_maps_resource_invalid_grant_to_resource_authorization() -> Result<()> {
    let idp_server = MockServer::start().await;
    let resource_server = MockServer::start().await;
    let issuer = format!("{}/idp", idp_server.uri());
    let audience = format!("{}/as", resource_server.uri());
    let resource = format!("{}/mcp", resource_server.uri());
    let jag = jag_response(&claims(&issuer, &audience, &resource));
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(jag))
        .mount(&idp_server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "invalid_grant",
            "error_description": "secret-resource-sentinel"
        })))
        .mount(&resource_server)
        .await;
    let error = exchange_id_jag(EmaIdJagExchangeRequest {
        resource: &resource,
        scopes: &[],
        mcp_client_id: "mcp-client",
        authorization_server_issuer: &audience,
        authorization_server_token_endpoint: &format!("{audience}/token"),
        idp_token_endpoint: &format!("{issuer}/token"),
        idp_issuer: &issuer,
        idp_client_id: "idp-client",
        refresh_token: "secret-refresh-token".to_string(),
        idp_http_client: http_client(),
        resource_http_client: http_client(),
    })
    .await
    .expect_err("resource invalid grant should fail");
    assert_eq!(
        error.downcast_ref::<EmaAuthFailure>(),
        Some(&EmaAuthFailure::InvalidGrant {
            grant_source: EmaInvalidGrantSource::ResourceAuthorization,
        })
    );
    assert!(!format!("{error:#}").contains("secret-resource-sentinel"));
    Ok(())
}
