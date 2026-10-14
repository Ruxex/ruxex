use ruxex_utils_absolute_path::test_support::PathExt;
use pretty_assertions::assert_eq;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use uuid::Uuid;

use super::*;

#[tokio::test]
async fn sqlite_sink_filters_noisy_targets_without_dropping_useful_diagnostics() {
    let ruxex_home =
        std::env::temp_dir().join(format!("ruxex-state-log-db-filter-{}", Uuid::new_v4()));
    let _cleanup = scopeguard::guard(ruxex_home.clone(), |ruxex_home| {
        let _ = std::fs::remove_dir_all(ruxex_home);
    });
    let runtime = StateRuntime::init(crate::SqliteConfig::new_for_testing(
        ruxex_home.as_path().abs(),
    ))
    .await
    .expect("initialize runtime");
    let layer = start(runtime.clone());

    let guard = tracing_subscriber::registry()
        .with(layer.clone().with_filter(default_filter()))
        .set_default();

    tracing::trace!(target: "opentelemetry_sdk", "dropped-trace");
    tracing::debug!(target: "opentelemetry_sdk", "dropped-debug");
    tracing::info!(target: "opentelemetry_sdk", "retained-info");
    tracing::warn!(target: "sqlx::query", "dropped-slow-query-warning");
    tracing::warn!(target: "sqlx::pool::acquire", "dropped-slow-acquire-warning");
    tracing::warn!(target: "sqlx::other", "dropped-sqlx-warning");
    tracing::info!(target: "sqlx_application", "retained-unrelated-target");
    tracing::debug!(target: "opentelemetry-otlp", "dropped-otlp-export");
    tracing::error!(target: "opentelemetry-http", "dropped-http-export");
    tracing::trace!(target: "h2::proto::streams", "dropped-http2-trace");
    tracing::debug!(target: "tonic::transport::channel", "dropped-grpc-transport-debug");
    tracing::debug!(target: "tower::buffer::worker", "dropped-grpc-buffer-debug");
    tracing::warn!(
        target: "h2::proto::ping_pong",
        "recv PING ack that we never sent: {:?}",
        "synthetic-ack"
    );
    tracing::warn!(target: "h2::proto::ping_pong", "retained-http2-ping-warning");
    tracing::warn!(target: "h2::proto::streams", "retained-http2-warning");
    tracing::debug!(target: "rmcp::transport", "dropped-rmcp-debug");
    tracing::info!(target: "rmcp::transport", "retained-rmcp-info");
    tracing::debug!(
        target: "ruxex_rmcp_client::oauth",
        "dropped-ruxex-rmcp-client-debug"
    );
    tracing::info!(
        target: "ruxex_rmcp_client::oauth",
        "retained-ruxex-rmcp-client-info"
    );
    tracing::trace!(target: "ruxex_http_client::transport", "dropped-request-body");
    tracing::debug!(target: "ruxex_http_client::transport", "retained-request-diagnostic");
    tracing::trace!(target: "ruxex_api::sse", "dropped-sse-parent");
    tracing::trace!(target: "ruxex_api::sse::responses", "dropped-sse-payload");
    tracing::debug!(target: "ruxex_api::sse::responses", "retained-sse-diagnostic");
    tracing::trace!(target: "ruxex_state", "retained-trace");
    tracing::trace!(
        target: "ruxex_tui::streaming::controller",
        "dropped-controller-trace"
    );
    tracing::debug!(
        target: "ruxex_tui::streaming::controller",
        "retained-controller-debug"
    );
    tracing::trace!(
        target: "ruxex_tui::streaming::table_holdback",
        "dropped-table-holdback-trace"
    );
    tracing::debug!(
        target: "ruxex_tui::streaming::table_holdback",
        "retained-table-holdback-debug"
    );
    tracing::trace!(
        target: "ruxex_tui::streaming::commit_tick",
        "retained-commit-tick-trace"
    );
    tracing::trace!(
        target: "ruxex_api::responses_websocket_timing",
        payload = "complete timing payload",
        "dropped-websocket-timing"
    );

    layer.flush().await;
    drop(guard);

    let logs = runtime
        .query_logs(&crate::LogQuery::default())
        .await
        .expect("query logs after flush");
    assert_eq!(
        logs.iter()
            .map(|row| (
                row.level.as_str(),
                row.target.as_str(),
                row.message.as_deref()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("INFO", "opentelemetry_sdk", Some("retained-info")),
            (
                "INFO",
                "sqlx_application",
                Some("retained-unrelated-target")
            ),
            (
                "WARN",
                "h2::proto::ping_pong",
                Some("retained-http2-ping-warning")
            ),
            ("WARN", "h2::proto::streams", Some("retained-http2-warning")),
            ("INFO", "rmcp::transport", Some("retained-rmcp-info")),
            (
                "INFO",
                "ruxex_rmcp_client::oauth",
                Some("retained-ruxex-rmcp-client-info")
            ),
            (
                "DEBUG",
                "ruxex_http_client::transport",
                Some("retained-request-diagnostic")
            ),
            (
                "DEBUG",
                "ruxex_api::sse::responses",
                Some("retained-sse-diagnostic")
            ),
            ("TRACE", "ruxex_state", Some("retained-trace")),
            (
                "DEBUG",
                "ruxex_tui::streaming::controller",
                Some("retained-controller-debug"),
            ),
            (
                "DEBUG",
                "ruxex_tui::streaming::table_holdback",
                Some("retained-table-holdback-debug"),
            ),
            (
                "TRACE",
                "ruxex_tui::streaming::commit_tick",
                Some("retained-commit-tick-trace"),
            ),
        ]
    );
}
