use std::sync::Arc;

use crate::config_manager::ConfigManager;
use crate::config_manager_service::ConfigManagerError;
use crate::error_code::internal_error;
use crate::error_code::invalid_request;
use crate::outgoing_message::OutgoingMessageSender;
use genex_app_server_protocol::AllowDenyRequirement;
use genex_app_server_protocol::BrowserUseAccessApprovalLifetime;
use genex_app_server_protocol::BrowserUseOriginPolicy;
use genex_app_server_protocol::BrowserUseRequirements;
use genex_app_server_protocol::CliAuthCredentialsStoreMode;
use genex_app_server_protocol::ClientResponsePayload;
use genex_app_server_protocol::ComputerUseMacosRequirements;
use genex_app_server_protocol::ComputerUseRequirements;
use genex_app_server_protocol::ConfigBatchWriteParams;
use genex_app_server_protocol::ConfigReadParams;
use genex_app_server_protocol::ConfigReadResponse;
use genex_app_server_protocol::ConfigRequirements;
use genex_app_server_protocol::ConfigRequirementsReadResponse;
use genex_app_server_protocol::ConfigValueWriteParams;
use genex_app_server_protocol::ConfigWriteErrorCode;
use genex_app_server_protocol::ConfigWriteResponse;
use genex_app_server_protocol::ConfiguredHookHandler;
use genex_app_server_protocol::ConfiguredHookMatcherGroup;
use genex_app_server_protocol::InAppBrowserRequirements;
use genex_app_server_protocol::JSONRPCErrorError;
use genex_app_server_protocol::ManagedHooksRequirements;
use genex_app_server_protocol::ModelProviderAuthField;
use genex_app_server_protocol::ModelProviderCapabilitiesReadResponse;
use genex_app_server_protocol::ModelProviderReadResponse;
use genex_app_server_protocol::ModelsRequirements;
use genex_app_server_protocol::NetworkDomainPermission;
use genex_app_server_protocol::NetworkRequirements;
use genex_app_server_protocol::NetworkUnixSocketPermission;
use genex_app_server_protocol::NewThreadModelDefaults;
use genex_app_server_protocol::SandboxMode;
use genex_config::ConfigRequirementsToml;
use genex_config::HookEventsToml;
use genex_config::HookHandlerConfig as CoreHookHandlerConfig;
use genex_config::ManagedHooksRequirementsToml;
use genex_config::MatcherGroup as CoreMatcherGroup;
use genex_config::ResidencyRequirement as CoreResidencyRequirement;
use genex_config::SandboxModeRequirement as CoreSandboxModeRequirement;
use genex_core::ThreadManager;
use genex_features::feature_for_key;
use genex_protocol::config_types::ForcedLoginMethod;
use genex_protocol::config_types::WebSearchMode;
use serde_json::json;
use std::path::PathBuf;

const FEATURE_CONFIG_KEYS_EXPOSED_IN_CONFIG_READ: &[&str] = &[
    "api_key_model_discovery",
    "auth_elicitation",
    "mcp_2026_07_28",
    "mentions_v2",
    "tool_suggest",
];

#[derive(Clone)]
pub(crate) struct ConfigRequestProcessor {
    outgoing: Arc<OutgoingMessageSender>,
    config_manager: ConfigManager,
    thread_manager: Arc<ThreadManager>,
}

impl ConfigRequestProcessor {
    pub(crate) fn new(
        outgoing: Arc<OutgoingMessageSender>,
        config_manager: ConfigManager,
        thread_manager: Arc<ThreadManager>,
    ) -> Self {
        Self {
            outgoing,
            config_manager,
            thread_manager,
        }
    }

    pub(crate) async fn read(
        &self,
        params: ConfigReadParams,
    ) -> Result<ConfigReadResponse, JSONRPCErrorError> {
        let fallback_cwd = params.cwd.as_ref().map(PathBuf::from);
        let mut response = self.config_manager.read(params).await.map_err(map_error)?;
        let config = self.load_latest_config(fallback_cwd).await?;
        for feature_key in FEATURE_CONFIG_KEYS_EXPOSED_IN_CONFIG_READ {
            let Some(feature) = feature_for_key(feature_key) else {
                continue;
            };
            let features = response
                .config
                .additional
                .entry("features".to_string())
                .or_insert_with(|| json!({}));
            if !features.is_object() {
                *features = json!({});
            }
            if let Some(features) = features.as_object_mut() {
                features.insert(
                    (*feature_key).to_string(),
                    json!(config.features.enabled(feature)),
                );
            }
        }
        Ok(response)
    }

    pub(crate) async fn config_requirements_read(
        &self,
    ) -> Result<ConfigRequirementsReadResponse, JSONRPCErrorError> {
        let requirements = self
            .config_manager
            .read_requirements()
            .await
            .map_err(map_error)?;
        let requirements = map_requirements_to_api(
            requirements,
            self.thread_manager
                .credential_manager()
                .allowed_login_methods(),
        );

        Ok(ConfigRequirementsReadResponse { requirements })
    }

    pub(crate) async fn value_write(
        &self,
        params: ConfigValueWriteParams,
    ) -> Result<ClientResponsePayload, JSONRPCErrorError> {
        self.handle_config_mutation_result(self.write_value(params).await)
            .await
            .map(ClientResponsePayload::ConfigValueWrite)
    }

    pub(crate) async fn batch_write(
        &self,
        params: ConfigBatchWriteParams,
    ) -> Result<ClientResponsePayload, JSONRPCErrorError> {
        let session_defaults_only = !params.edits.is_empty()
            && params.edits.iter().all(|edit| {
                matches!(
                    edit.key_path.as_str(),
                    "model"
                        | "model_reasoning_effort"
                        | "plan_mode_reasoning_effort"
                        | "service_tier"
                )
            });
        let should_reload = params.reload_user_config;
        let response = self.batch_write_inner(params).await?;
        if !session_defaults_only {
            self.handle_config_mutation().await;
            if should_reload {
                reload_user_config(&self.config_manager, &self.thread_manager).await;
            }
        }
        Ok(ClientResponsePayload::ConfigBatchWrite(response))
    }

    pub(crate) async fn model_provider_capabilities_read(
        &self,
    ) -> Result<ModelProviderCapabilitiesReadResponse, JSONRPCErrorError> {
        let _config = self.load_latest_config(/*fallback_cwd*/ None).await?;
        let provider = self.thread_manager.model_provider_contract();
        let capabilities = provider.capabilities();
        Ok(ModelProviderCapabilitiesReadResponse {
            namespace_tools: capabilities.namespace_tools,
            web_search: capabilities.web_search,
        })
    }

    pub(crate) async fn model_provider_read(
        &self,
    ) -> Result<ModelProviderReadResponse, JSONRPCErrorError> {
        let _config = self.load_latest_config(/*fallback_cwd*/ None).await?;
        let provider = self.thread_manager.model_provider_contract();
        let descriptor = provider.descriptor();
        let account_state = provider
            .account_state()
            .map_err(|err| invalid_request(err.to_string()))?;
        Ok(ModelProviderReadResponse {
            id: descriptor.provider_kind.clone(),
            name: descriptor.provider_kind,
            auth_status: account_state.auth_status,
            auth_fields: provider
                .auth_fields()
                .into_iter()
                .map(|field| ModelProviderAuthField {
                    id: field.id,
                    label: field.label,
                    secret: field.secret,
                })
                .collect(),
        })
    }

    pub(crate) async fn handle_config_mutation(&self) {
        self.thread_manager.plugins_manager().clear_cache();
        self.thread_manager.skills_service().clear_cache();
    }

    async fn handle_config_mutation_result<T>(
        &self,
        result: std::result::Result<T, JSONRPCErrorError>,
    ) -> Result<T, JSONRPCErrorError> {
        let response = result?;
        self.handle_config_mutation().await;
        Ok(response)
    }

    async fn load_latest_config(
        &self,
        fallback_cwd: Option<PathBuf>,
    ) -> Result<genex_core::config::Config, JSONRPCErrorError> {
        self.config_manager
            .load_latest_config(fallback_cwd)
            .await
            .map_err(|err| {
                internal_error(format!(
                    "failed to resolve feature override precedence: {err}"
                ))
            })
    }

    async fn write_value(
        &self,
        params: ConfigValueWriteParams,
    ) -> Result<ConfigWriteResponse, JSONRPCErrorError> {
        let _pending_changes = genex_core_plugins::toggles::collect_plugin_enabled_candidates(
            [(&params.key_path, &params.value)].into_iter(),
        );
        let response = self
            .config_manager
            .write_value(params)
            .await
            .map_err(map_error)?;
        Ok(response)
    }

    async fn batch_write_inner(
        &self,
        params: ConfigBatchWriteParams,
    ) -> Result<ConfigWriteResponse, JSONRPCErrorError> {
        let _pending_changes = genex_core_plugins::toggles::collect_plugin_enabled_candidates(
            params
                .edits
                .iter()
                .map(|edit| (&edit.key_path, &edit.value)),
        );
        let response = self
            .config_manager
            .batch_write(params)
            .await
            .map_err(map_error)?;
        Ok(response)
    }
}

pub(super) async fn reload_user_config(
    config_manager: &ConfigManager,
    thread_manager: &ThreadManager,
) {
    if let Err(err) = config_manager
        .load_latest_config(/*fallback_cwd*/ None)
        .await
    {
        tracing::warn!("failed to rebuild user config for runtime refresh: {err}");
        return;
    }
    let thread_ids = thread_manager.list_thread_ids().await;
    for thread_id in thread_ids {
        let Ok(thread) = thread_manager.get_thread(thread_id).await else {
            continue;
        };
        let current_config = thread.config().await;
        let next_config = match config_manager
            .load_latest_config_with_session_layers(
                &current_config.config_layer_stack,
                &current_config.cwd,
            )
            .await
        {
            Ok(config) => config,
            Err(err) => {
                tracing::warn!(%thread_id, %err, "failed to reload thread configuration");
                continue;
            }
        };
        // Keep runtime refresh state off the request dispatcher's stack.
        Box::pin(thread.refresh_runtime_config(next_config)).await;
    }
}

fn map_requirements_to_api(
    requirements: Option<ConfigRequirementsToml>,
    allowed_login_methods: Vec<ForcedLoginMethod>,
) -> Option<ConfigRequirements> {
    let requirements = match requirements {
        Some(requirements) => requirements,
        None if allowed_login_methods == [ForcedLoginMethod::Api, ForcedLoginMethod::Managed] => {
            return None;
        }
        None => ConfigRequirementsToml::default(),
    };
    Some(ConfigRequirements {
        allowed_login_methods: Some(allowed_login_methods),
        application: requirements.application.map(|application| {
            genex_app_server_protocol::ApplicationRequirements {
                network: application.network.map(|network| {
                    genex_app_server_protocol::ApplicationNetworkRequirements {
                        enabled: network.enabled,
                        domains: network
                            .domains
                            .into_iter()
                            .map(|(domain, permission)| {
                                (domain, map_network_domain_permission_to_api(permission))
                            })
                            .collect(),
                    }
                }),
            }
        }),
        cli_auth_credentials_store: requirements.cli_auth_credentials_store.map(
            |mode| match mode {
                genex_config::types::AuthCredentialsStoreMode::File => {
                    CliAuthCredentialsStoreMode::File
                }
                genex_config::types::AuthCredentialsStoreMode::Keyring => {
                    CliAuthCredentialsStoreMode::Keyring
                }
                genex_config::types::AuthCredentialsStoreMode::Auto => {
                    CliAuthCredentialsStoreMode::Auto
                }
                genex_config::types::AuthCredentialsStoreMode::Ephemeral => {
                    CliAuthCredentialsStoreMode::Ephemeral
                }
            },
        ),
        managed_auth_base_url: requirements.managed_auth_base_url,
        additional_developer_instructions: requirements.additional_developer_instructions,
        allowed_approval_policies: requirements.allowed_approval_policies.map(|policies| {
            policies
                .into_iter()
                .map(genex_app_server_protocol::AskForApproval::from)
                .collect()
        }),
        allowed_sandbox_modes: requirements.allowed_sandbox_modes.map(|modes| {
            modes
                .into_iter()
                .filter_map(map_sandbox_mode_requirement_to_api)
                .collect()
        }),
        allowed_permission_profiles: requirements.allowed_permission_profiles,
        default_permissions: requirements.default_permissions,
        allowed_web_search_modes: requirements.allowed_web_search_modes.map(|modes| {
            let mut normalized = modes
                .into_iter()
                .map(Into::into)
                .collect::<Vec<WebSearchMode>>();
            if !normalized.contains(&WebSearchMode::Disabled) {
                normalized.push(WebSearchMode::Disabled);
            }
            normalized
        }),
        allow_managed_hooks_only: requirements.allow_managed_hooks_only,
        allow_browser_and_computer_use: requirements.allow_browser_and_computer_use,
        allow_appshots: requirements.allow_appshots,
        computer_use: requirements
            .computer_use
            .map(map_computer_use_requirements_to_api),
        browser_use: requirements
            .browser_use
            .map(map_browser_use_requirements_to_api),
        in_app_browser: requirements.in_app_browser.map(|in_app_browser| {
            InAppBrowserRequirements {
                allow_external_browser_settings_import: in_app_browser
                    .allow_external_browser_settings_import,
            }
        }),
        feature_requirements: requirements
            .feature_requirements
            .map(|requirements| requirements.entries),
        hooks: requirements.hooks.map(map_hooks_requirements_to_api),
        enforce_residency: requirements
            .enforce_residency
            .map(map_residency_requirement_to_api),
        network: requirements.network.map(map_network_requirements_to_api),
        models: requirements.models.map(|models| ModelsRequirements {
            new_thread: models.new_thread.map(|new_thread| NewThreadModelDefaults {
                model: new_thread.model,
                model_reasoning_effort: new_thread.model_reasoning_effort,
                service_tier: new_thread.service_tier,
            }),
        }),
        sqlite_home: requirements.sqlite_home.map(Into::into),
        log_dir: requirements.log_dir.map(Into::into),
        model_catalog_json: requirements.model_catalog_json.map(Into::into),
        allow_login_shell: requirements.allow_login_shell,
    })
}

fn map_computer_use_requirements_to_api(
    computer_use: genex_config::ComputerUseRequirementsToml,
) -> ComputerUseRequirements {
    ComputerUseRequirements {
        allow_locked_computer_use: computer_use.allow_locked_computer_use,
        allow_persistent_approval: computer_use.allow_persistent_approval,
        default_app_access: computer_use
            .default_app_access
            .map(map_allow_deny_requirement_to_api),
        macos: computer_use
            .macos
            .map(|macos| ComputerUseMacosRequirements {
                bundle_ids: macos.bundle_ids.map(|bundle_ids| {
                    bundle_ids
                        .into_iter()
                        .map(|(bundle_id, requirement)| {
                            (bundle_id, map_allow_deny_requirement_to_api(requirement))
                        })
                        .collect()
                }),
            }),
    }
}

fn map_browser_use_requirements_to_api(
    browser_use: genex_config::BrowserUseRequirementsToml,
) -> BrowserUseRequirements {
    BrowserUseRequirements {
        allow_webmcp: browser_use.allow_webmcp,
        allow_history_access: browser_use.allow_history_access,
        disable_auto_review: browser_use.disable_auto_review,
        allow_global_persistent_approval: browser_use.allow_global_persistent_approval,
        default_origin_policy: browser_use
            .default_origin_policy
            .map(map_browser_use_origin_policy_to_api),
        origins: browser_use.origins.map(|origins| {
            origins
                .into_iter()
                .map(|(pattern, policy)| (pattern, map_browser_use_origin_policy_to_api(policy)))
                .collect()
        }),
    }
}

fn map_browser_use_origin_policy_to_api(
    policy: genex_config::BrowserUseOriginPolicyToml,
) -> BrowserUseOriginPolicy {
    BrowserUseOriginPolicy {
        access: policy.access.map(map_allow_deny_requirement_to_api),
        downloads: policy.downloads.map(map_allow_deny_requirement_to_api),
        uploads: policy.uploads.map(map_allow_deny_requirement_to_api),
        full_cdp_access: policy
            .full_cdp_access
            .map(map_allow_deny_requirement_to_api),
        auto_review: policy.auto_review.map(map_allow_deny_requirement_to_api),
        persistent_approval: policy.persistent_approval,
        access_approval_lifetime: policy
            .access_approval_lifetime
            .map(map_browser_use_access_approval_lifetime_to_api),
    }
}

fn map_allow_deny_requirement_to_api(
    requirement: genex_config::AllowDenyRequirementToml,
) -> AllowDenyRequirement {
    match requirement {
        genex_config::AllowDenyRequirementToml::Allow => AllowDenyRequirement::Allow,
        genex_config::AllowDenyRequirementToml::Deny => AllowDenyRequirement::Deny,
    }
}

fn map_browser_use_access_approval_lifetime_to_api(
    lifetime: genex_config::BrowserUseAccessApprovalLifetimeToml,
) -> BrowserUseAccessApprovalLifetime {
    match lifetime {
        genex_config::BrowserUseAccessApprovalLifetimeToml::Turn => {
            BrowserUseAccessApprovalLifetime::Turn
        }
        genex_config::BrowserUseAccessApprovalLifetimeToml::Thread => {
            BrowserUseAccessApprovalLifetime::Thread
        }
    }
}

fn map_hooks_requirements_to_api(hooks: ManagedHooksRequirementsToml) -> ManagedHooksRequirements {
    let ManagedHooksRequirementsToml { managed_dir, hooks } = hooks;
    let HookEventsToml {
        pre_tool_use,
        permission_request,
        post_tool_use,
        pre_compact,
        post_compact,
        session_start,
        session_end,
        user_prompt_submit,
        subagent_start,
        subagent_stop,
        stop,
        interrupt,
    } = hooks;

    ManagedHooksRequirements {
        managed_dir,
        pre_tool_use: map_hook_matcher_groups_to_api(pre_tool_use),
        permission_request: map_hook_matcher_groups_to_api(permission_request),
        post_tool_use: map_hook_matcher_groups_to_api(post_tool_use),
        pre_compact: map_hook_matcher_groups_to_api(pre_compact),
        post_compact: map_hook_matcher_groups_to_api(post_compact),
        session_start: map_hook_matcher_groups_to_api(session_start),
        session_end: map_hook_matcher_groups_to_api(session_end),
        user_prompt_submit: map_hook_matcher_groups_to_api(user_prompt_submit),
        subagent_start: map_hook_matcher_groups_to_api(subagent_start),
        subagent_stop: map_hook_matcher_groups_to_api(subagent_stop),
        stop: map_hook_matcher_groups_to_api(stop),
        interrupt: map_hook_matcher_groups_to_api(interrupt),
    }
}

fn map_hook_matcher_groups_to_api(
    groups: Vec<CoreMatcherGroup>,
) -> Vec<ConfiguredHookMatcherGroup> {
    groups
        .into_iter()
        .map(map_hook_matcher_group_to_api)
        .collect()
}

fn map_hook_matcher_group_to_api(group: CoreMatcherGroup) -> ConfiguredHookMatcherGroup {
    ConfiguredHookMatcherGroup {
        matcher: group.matcher,
        hooks: group
            .hooks
            .into_iter()
            .map(map_hook_handler_to_api)
            .collect(),
    }
}

fn map_hook_handler_to_api(handler: CoreHookHandlerConfig) -> ConfiguredHookHandler {
    match handler {
        CoreHookHandlerConfig::Command {
            command,
            timeout_sec,
            r#async,
            status_message,
            additional_context_limit,
        } => ConfiguredHookHandler::Command {
            command,
            timeout_sec,
            r#async,
            status_message,
            additional_context_limit,
        },
        CoreHookHandlerConfig::McpTool {
            server,
            tool,
            input,
            timeout_sec,
            status_message,
        } => ConfiguredHookHandler::McpTool {
            server,
            tool,
            input,
            timeout_sec,
            status_message,
        },
        CoreHookHandlerConfig::Prompt {} => ConfiguredHookHandler::Prompt {},
        CoreHookHandlerConfig::Agent {} => ConfiguredHookHandler::Agent {},
    }
}

fn map_sandbox_mode_requirement_to_api(mode: CoreSandboxModeRequirement) -> Option<SandboxMode> {
    match mode {
        CoreSandboxModeRequirement::ReadOnly => Some(SandboxMode::ReadOnly),
        CoreSandboxModeRequirement::WorkspaceWrite => Some(SandboxMode::WorkspaceWrite),
        CoreSandboxModeRequirement::DangerFullAccess => Some(SandboxMode::DangerFullAccess),
        CoreSandboxModeRequirement::ExternalSandbox => None,
    }
}

fn map_residency_requirement_to_api(
    residency: CoreResidencyRequirement,
) -> genex_app_server_protocol::ResidencyRequirement {
    match residency {
        CoreResidencyRequirement::Us => genex_app_server_protocol::ResidencyRequirement::Us,
    }
}

fn map_network_requirements_to_api(
    network: genex_config::NetworkRequirementsToml,
) -> NetworkRequirements {
    let allowed_domains = network
        .domains
        .as_ref()
        .and_then(genex_config::NetworkDomainPermissionsToml::allowed_domains);
    let denied_domains = network
        .domains
        .as_ref()
        .and_then(genex_config::NetworkDomainPermissionsToml::denied_domains);
    let allow_unix_sockets = network
        .unix_sockets
        .as_ref()
        .map(genex_config::NetworkUnixSocketPermissionsToml::allow_unix_sockets)
        .filter(|entries| !entries.is_empty());

    NetworkRequirements {
        enabled: network.enabled,
        http_port: network.http_port,
        socks_port: network.socks_port,
        allow_upstream_proxy: network.allow_upstream_proxy,
        dangerously_allow_non_loopback_proxy: network.dangerously_allow_non_loopback_proxy,
        dangerously_allow_all_unix_sockets: network.dangerously_allow_all_unix_sockets,
        domains: network.domains.map(|domains| {
            domains
                .entries
                .into_iter()
                .map(|(pattern, permission)| {
                    (pattern, map_network_domain_permission_to_api(permission))
                })
                .collect()
        }),
        managed_allowed_domains_only: network.managed_allowed_domains_only,
        allowed_domains,
        denied_domains,
        unix_sockets: network.unix_sockets.map(|unix_sockets| {
            unix_sockets
                .entries
                .into_iter()
                .map(|(path, permission)| {
                    (path, map_network_unix_socket_permission_to_api(permission))
                })
                .collect()
        }),
        allow_unix_sockets,
        allow_local_binding: network.allow_local_binding,
    }
}

fn map_network_domain_permission_to_api(
    permission: genex_config::NetworkDomainPermissionToml,
) -> NetworkDomainPermission {
    match permission {
        genex_config::NetworkDomainPermissionToml::Allow => NetworkDomainPermission::Allow,
        genex_config::NetworkDomainPermissionToml::Deny => NetworkDomainPermission::Deny,
    }
}

fn map_network_unix_socket_permission_to_api(
    permission: genex_config::NetworkUnixSocketPermissionToml,
) -> NetworkUnixSocketPermission {
    match permission {
        genex_config::NetworkUnixSocketPermissionToml::Allow => NetworkUnixSocketPermission::Allow,
        genex_config::NetworkUnixSocketPermissionToml::Deny => NetworkUnixSocketPermission::Deny,
    }
}

pub(super) fn map_error(err: ConfigManagerError) -> JSONRPCErrorError {
    if let Some(code) = err.write_error_code() {
        return config_write_error(code, err.to_string());
    }

    internal_error(err.to_string())
}

fn config_write_error(code: ConfigWriteErrorCode, message: impl Into<String>) -> JSONRPCErrorError {
    let mut error = invalid_request(message);
    error.data = Some(json!({
        "config_write_error_code": code,
    }));
    error
}
