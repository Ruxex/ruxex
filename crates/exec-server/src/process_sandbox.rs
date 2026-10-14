use std::collections::HashMap;
use std::sync::Arc;

use crate::process_telemetry::ProcessTelemetry;
use ruxex_exec_server_protocol::JSONRPCErrorError;
use ruxex_network_proxy::CUSTOM_CA_ENV_KEYS;
use ruxex_network_proxy::ManagedNetworkSandboxContext;
use ruxex_network_proxy::ManagedProxyRouting;
use ruxex_network_proxy::NetworkPolicyAuditObserver;
use ruxex_network_proxy::NetworkPolicyDecider;
use ruxex_network_proxy::NetworkProxy;
use ruxex_network_proxy::NetworkProxyHandle;
use ruxex_network_proxy::NetworkProxyState;
use ruxex_network_proxy::RemoteNetworkProxyLaunchConfig;
use ruxex_network_proxy::is_managed_mitm_ca_trust_bundle_path;
use ruxex_protocol::models::PermissionProfile;
use ruxex_sandboxing::SandboxCommand;
use ruxex_sandboxing::SandboxDirectSpawnTransformRequest;
use ruxex_sandboxing::SandboxManager;
use ruxex_sandboxing::SandboxTransformRequest;
use ruxex_sandboxing::SandboxType;
use ruxex_sandboxing::with_managed_mitm_ca_readable_root;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_path_uri::PathUri;

use crate::ExecServerRuntimePaths;
#[cfg(unix)]
use crate::RUXEX_ARG0_EXEC_HELPER_ARG1;
use crate::protocol::ExecParams;
use crate::rpc::internal_error;
use crate::rpc::invalid_params;
use crate::sandbox_selection::select_sandbox;

pub(crate) struct PreparedExecRequest {
    pub(crate) command: Vec<String>,
    pub(crate) cwd: AbsolutePathBuf,
    pub(crate) env: HashMap<String, String>,
    pub(crate) arg0: Option<String>,
    pub(crate) sandbox: SandboxType,
    pub(crate) network_proxy_handle: Option<NetworkProxyHandle>,
}

pub(crate) async fn prepare_exec_request_with_telemetry(
    params: &ExecParams,
    env: HashMap<String, String>,
    runtime_paths: Option<&ExecServerRuntimePaths>,
    network_policy_decider: Option<Arc<dyn NetworkPolicyDecider>>,
    network_policy_audit_observer: Option<NetworkPolicyAuditObserver>,
    telemetry: &ProcessTelemetry,
) -> Result<PreparedExecRequest, JSONRPCErrorError> {
    let network_proxy = params.network_proxy.as_ref();

    let (env, managed_network, network_proxy_handle) = prepare_managed_network(
        params.managed_network.as_ref(),
        network_proxy,
        ManagedProxyRouting::DedicatedListeners,
        env,
        network_policy_decider,
        network_policy_audit_observer,
        telemetry,
    )
    .await?;
    let Some(sandbox_context) = params.sandbox.as_ref() else {
        return Ok(PreparedExecRequest {
            command: params.argv.clone(),
            cwd: native_path(&params.cwd, "cwd")?,
            env,
            arg0: params.arg0.clone(),
            sandbox: SandboxType::None,
            network_proxy_handle,
        });
    };
    let runtime_paths = runtime_paths
        .ok_or_else(|| invalid_params("sandbox runtime paths are not configured".to_string()))?;
    sandbox_context
        .validate_file_system_paths_for_current_host()
        .map_err(|err| invalid_params(err.to_string()))?;
    // TODO(jif): Transport permissions before orchestrator-local paths are materialized,
    // then resolve executor-local helper and workspace paths here.
    let permissions = sandbox_context.permissions.clone();
    let sandbox_policy_cwd = &sandbox_context.cwd;
    let native_sandbox_policy_cwd = native_path(sandbox_policy_cwd, "sandbox cwd")?;
    let native_workspace_roots = sandbox_context
        .workspace_roots
        .iter()
        .map(|root| native_path(root, "sandbox workspace root"))
        .collect::<Result<Vec<_>, _>>()?;
    let workspace_roots = native_workspace_roots.as_slice();
    let permissions = permissions.materialize_project_roots_with_workspace_roots(workspace_roots);
    let managed_mitm_ca_trust_bundle_path = managed_network.as_ref().and_then(|_| {
        CUSTOM_CA_ENV_KEYS.iter().find_map(|key| {
            let path = env.get(*key)?;
            if !is_managed_mitm_ca_trust_bundle_path(path) {
                return None;
            }
            AbsolutePathBuf::from_absolute_path(path).ok()
        })
    });
    let permissions = with_managed_mitm_ca_readable_root(
        permissions,
        managed_mitm_ca_trust_bundle_path.as_ref(),
        native_sandbox_policy_cwd.as_path(),
    );
    #[cfg(unix)]
    let (file_system_policy, network_policy) = permissions.to_runtime_permissions();
    #[cfg(unix)]
    let sandbox_helper_paths = params
        .arg0
        .iter()
        .map(|_| runtime_paths.ruxex_self_exe.clone())
        .collect::<Vec<_>>();
    // Bubblewrap launches the configured helper, which may re-enter this executable to apply
    // seccomp, so the outer filesystem sandbox must expose both paths.
    #[cfg(target_os = "linux")]
    let sandbox_helper_paths = {
        let mut sandbox_helper_paths = sandbox_helper_paths;
        if !sandbox_helper_paths.contains(&runtime_paths.ruxex_self_exe) {
            sandbox_helper_paths.push(runtime_paths.ruxex_self_exe.clone());
        }
        sandbox_helper_paths.extend(runtime_paths.ruxex_linux_sandbox_exe.iter().cloned());
        sandbox_helper_paths
    };
    #[cfg(unix)]
    let file_system_policy = file_system_policy
        .with_additional_readable_roots(native_sandbox_policy_cwd.as_path(), &sandbox_helper_paths);
    #[cfg(unix)]
    let permissions = PermissionProfile::from_runtime_permissions_with_enforcement(
        permissions.enforcement(),
        &file_system_policy,
        network_policy,
    );
    let sandbox_manager = SandboxManager::new();
    #[cfg(target_os = "macos")]
    let sandbox_manager = sandbox_manager
        .with_allowed_symlinked_ruxex_home(runtime_paths.allowed_symlinked_ruxex_home.clone());
    let sandbox = select_sandbox(
        &sandbox_manager,
        &permissions,
        params.enforce_managed_network,
    );
    if sandbox == SandboxType::None {
        return Err(invalid_params(
            "sandbox intent cannot be enforced on this executor".to_string(),
        ));
    }
    let (program, args) = params
        .argv
        .split_first()
        .ok_or_else(|| invalid_params("argv must not be empty".to_string()))?;
    #[cfg(unix)]
    let (program, args) = params.arg0.as_ref().map_or_else(
        || (program.into(), args.to_vec()),
        |arg0| {
            let mut helper_args = Vec::with_capacity(params.argv.len() + 2);
            helper_args.push(RUXEX_ARG0_EXEC_HELPER_ARG1.to_string());
            helper_args.push(arg0.clone());
            helper_args.extend(params.argv.iter().cloned());
            (
                runtime_paths
                    .ruxex_self_exe
                    .as_path()
                    .as_os_str()
                    .to_owned(),
                helper_args,
            )
        },
    );
    #[cfg(not(unix))]
    let (program, args) = (program.into(), args.to_vec());
    let transform_request = SandboxDirectSpawnTransformRequest {
        transform: SandboxTransformRequest {
            command: SandboxCommand {
                program,
                args,
                cwd: params.cwd.clone(),
                env,
                managed_network,
                additional_permissions: None,
            },
            permissions: &permissions,
            sandbox,
            enforce_managed_network: params.enforce_managed_network,
            environment_id: None,
            network: None,
            sandbox_policy_cwd,
            sandbox_exe: runtime_paths.ruxex_linux_sandbox_exe.as_deref(),
            use_legacy_landlock: sandbox_context.use_legacy_landlock,
        },
    };
    let request = sandbox_manager
        .transform_for_direct_spawn(transform_request)
        .map_err(|err| invalid_params(format!("failed to prepare process sandbox: {err}")))?;
    Ok(PreparedExecRequest {
        command: request.command,
        cwd: native_path(&request.cwd, "cwd")?,
        env: request.env,
        arg0: request.arg0,
        sandbox: request.sandbox,
        network_proxy_handle,
    })
}

async fn prepare_managed_network(
    managed_network: Option<&ManagedNetworkSandboxContext>,
    network_proxy: Option<&RemoteNetworkProxyLaunchConfig>,
    routing: ManagedProxyRouting,
    env: HashMap<String, String>,
    network_policy_decider: Option<Arc<dyn NetworkPolicyDecider>>,
    network_policy_audit_observer: Option<NetworkPolicyAuditObserver>,
    telemetry: &ProcessTelemetry,
) -> Result<
    (
        HashMap<String, String>,
        Option<ManagedNetworkSandboxContext>,
        Option<NetworkProxyHandle>,
    ),
    JSONRPCErrorError,
> {
    let Some(network_proxy) = network_proxy.cloned() else {
        return Ok((env, managed_network.cloned(), None));
    };
    let mut state = NetworkProxyState::from_remote_launch_config(network_proxy)
        .map_err(|err| invalid_params(format!("invalid network proxy config: {err}")))?;
    if let Some(observer) = network_policy_audit_observer {
        state.set_policy_audit_observer(observer);
    }
    if let Some(launch_context) = &telemetry.launch_context {
        state.set_launch_span_context(launch_context.clone());
    }
    state.set_process_log_metadata(ruxex_network_proxy::NetworkProxyProcessLogMetadata {
        thread_id: telemetry.thread_id.clone(),
        tool_call_id: telemetry.tool_call_id.clone(),
        executor_identity: None,
    });
    let mut builder = NetworkProxy::builder()
        .state(Arc::new(state))
        .managed_proxy_routing(routing);
    if let Some(network_policy_decider) = network_policy_decider {
        builder = builder.policy_decider_arc(network_policy_decider);
    }
    let proxy = builder
        .build()
        .await
        .map_err(|err| internal_error(format!("failed to build executor network proxy: {err}")))?;
    let handle = proxy
        .run()
        .await
        .map_err(|err| internal_error(format!("failed to start executor network proxy: {err}")))?;
    let prepared = proxy
        .prepare_for_optional_environment(env, /*environment_id*/ None)
        .map_err(|err| {
            internal_error(format!("failed to prepare executor network proxy: {err}"))
        })?;
    Ok((prepared.env, Some(prepared.sandbox_context), Some(handle)))
}

fn native_path(path: &PathUri, label: &str) -> Result<AbsolutePathBuf, JSONRPCErrorError> {
    path.to_abs_path().map_err(|err| {
        invalid_params(format!(
            "{label} URI `{path}` is not valid on this exec-server host: {err}"
        ))
    })
}

#[cfg(test)]
#[path = "process_sandbox_tests.rs"]
mod tests;
