use crate::tools::sandboxing::ToolCtx;
use codex_core_plugins::PluginMetricsSidecar;
use codex_exec_server::Environment;
use codex_protocol::items::ModelInvocationContext;
use codex_utils_path_uri::PathUri;

/// Creates a metrics sidecar for one plugin command.
pub(crate) async fn sidecar_for_command(
    ctx: &ToolCtx,
    command: &[String],
    cwd: &PathUri,
    environment: &Environment,
) -> Option<PluginMetricsSidecar> {
    let resolved = ctx
        .step_context
        .turn
        .plugin_metrics_operation_for_command(command, cwd, environment)
        .await?;
    if environment.is_remote() {
        PluginMetricsSidecar::create_remote(environment, resolved).await
    } else {
        PluginMetricsSidecar::create(resolved)
    }
}

/// Finishes a metrics sidecar and publishes any valid rows.
pub(crate) async fn finish_and_track_measurements(
    metrics_sidecar: Option<PluginMetricsSidecar>,
    exit_code: i32,
    _session: &crate::session::session::Session,
    _turn: &crate::session::turn_context::TurnContext,
    _model_context: &ModelInvocationContext,
    _item_id: &str,
) {
    let Some(metrics_sidecar) = metrics_sidecar else {
        return;
    };
    let _ = metrics_sidecar.finish(exit_code).await;
}
