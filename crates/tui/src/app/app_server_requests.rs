use std::collections::HashMap;
use std::collections::VecDeque;

use super::App;
use crate::app_command::AppCommand;
use crate::app_server_approval_conversions::granted_permission_profile_from_request;
use crate::app_server_session::AppServerSession;
use ruxex_app_server_protocol::CommandExecutionRequestApprovalResponse;
use ruxex_app_server_protocol::FileChangeRequestApprovalResponse;
use ruxex_app_server_protocol::JSONRPCErrorError;
use ruxex_app_server_protocol::McpServerElicitationRequestResponse;
use ruxex_app_server_protocol::PermissionsRequestApprovalResponse;
use ruxex_app_server_protocol::RequestId as AppServerRequestId;
use ruxex_app_server_protocol::ServerRequest;
use ruxex_protocol::request_permissions::RequestPermissionProfile as CoreRequestPermissionProfile;

impl App {
    pub(super) async fn reject_app_server_request(
        &self,
        app_server_client: &AppServerSession,
        request_id: AppServerRequestId,
        reason: String,
    ) -> std::result::Result<(), String> {
        app_server_client
            .reject_server_request(
                request_id,
                JSONRPCErrorError {
                    code: -32000,
                    message: reason,
                    data: None,
                },
            )
            .await
            .map_err(|err| format!("failed to reject app-server request: {err}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppServerRequestResolution {
    pub(super) request_id: AppServerRequestId,
    pub(super) result: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UnsupportedAppServerRequest {
    pub(super) request_id: AppServerRequestId,
    pub(super) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedAppServerRequest {
    ExecApproval {
        thread_id: String,
        id: String,
    },
    FileChangeApproval {
        thread_id: String,
        id: String,
    },
    PermissionsApproval {
        thread_id: String,
        id: String,
    },
    UserInput {
        call_id: String,
    },
    McpElicitation {
        server_name: String,
        request_id: AppServerRequestId,
    },
}

#[derive(Debug, Default)]
pub(super) struct PendingAppServerRequests {
    exec_approvals: HashMap<(String, String), AppServerRequestId>,
    file_change_approvals: HashMap<(String, String), AppServerRequestId>,
    permissions_approvals: HashMap<(String, String), AppServerRequestId>,
    user_inputs: HashMap<String, VecDeque<PendingUserInputRequest>>,
    mcp_requests: HashMap<McpRequestKey, AppServerRequestId>,
    pub(super) user_verification: super::user_verification_requests::UserVerificationRequests,
}

impl PendingAppServerRequests {
    fn canonical_thread_id(thread_id: &str) -> String {
        ruxex_protocol::ThreadId::from_string(thread_id)
            .map(|thread_id| thread_id.to_string())
            .unwrap_or_else(|_| thread_id.to_string())
    }

    pub(super) fn clear(&mut self) {
        self.exec_approvals.clear();
        self.file_change_approvals.clear();
        self.permissions_approvals.clear();
        self.user_inputs.clear();
        self.mcp_requests.clear();
        self.user_verification.clear();
    }

    pub(super) fn cancel_thread_verification(&mut self, thread_id: &str) {
        for (server_name, request_id) in self.user_verification.cancel_thread(thread_id) {
            self.mcp_requests.remove(&McpRequestKey {
                server_name,
                request_id,
            });
        }
    }

    pub(super) fn note_server_request(
        &mut self,
        request: &ServerRequest,
    ) -> Option<UnsupportedAppServerRequest> {
        match request {
            ServerRequest::CommandExecutionRequestApproval { request_id, params } => {
                let approval_id = params
                    .approval_id
                    .clone()
                    .unwrap_or_else(|| params.item_id.clone());
                self.exec_approvals.insert(
                    (Self::canonical_thread_id(&params.thread_id), approval_id),
                    request_id.clone(),
                );
                None
            }
            ServerRequest::FileChangeRequestApproval { request_id, params } => {
                self.file_change_approvals.insert(
                    (
                        Self::canonical_thread_id(&params.thread_id),
                        params.item_id.clone(),
                    ),
                    request_id.clone(),
                );
                None
            }
            ServerRequest::PermissionsRequestApproval { request_id, params } => {
                // TODO(anp): Remove this duplicate validation once core permission paths remain
                // PathUri after crossing the app-server boundary. Native permission paths do not
                // yet have an ingress validation step, so validate them here before recording the
                // request as pending. Discovering an invalid path later in a UI delivery path
                // would leave the app-server RPC waiting without a clean rejection path.
                if let Err(err) = CoreRequestPermissionProfile::try_from(params.permissions.clone())
                {
                    return Some(UnsupportedAppServerRequest {
                        request_id: request_id.clone(),
                        message: format!("failed to localize requested filesystem paths: {err}"),
                    });
                }
                self.permissions_approvals.insert(
                    (
                        Self::canonical_thread_id(&params.thread_id),
                        params.item_id.clone(),
                    ),
                    request_id.clone(),
                );
                None
            }
            ServerRequest::ToolRequestUserInput { request_id, params } => {
                self.user_inputs
                    .entry(params.turn_id.clone())
                    .or_default()
                    .push_back(PendingUserInputRequest {
                        item_id: params.item_id.clone(),
                        request_id: request_id.clone(),
                    });
                None
            }
            ServerRequest::McpServerElicitationRequest { request_id, params } => {
                self.user_verification.note_request(request_id, params);
                self.mcp_requests.insert(
                    McpRequestKey {
                        server_name: params.server_name.clone(),
                        request_id: request_id.clone(),
                    },
                    request_id.clone(),
                );
                None
            }
            ServerRequest::DynamicToolCall { .. } => None,
            ServerRequest::AttestationGenerate { request_id, .. } => {
                Some(UnsupportedAppServerRequest {
                    request_id: request_id.clone(),
                    message: "Attestation generation is not available in TUI.".to_string(),
                })
            }
            ServerRequest::CurrentTimeRead { request_id, .. } => {
                Some(UnsupportedAppServerRequest {
                    request_id: request_id.clone(),
                    message: "External current time is not available in TUI.".to_string(),
                })
            }
        }
    }

    pub(super) fn take_resolution<T>(
        &mut self,
        thread_id: &str,
        op: T,
    ) -> Result<Option<AppServerRequestResolution>, String>
    where
        T: Into<AppCommand>,
    {
        let thread_id = Self::canonical_thread_id(thread_id);
        let op: AppCommand = match op.into() {
            AppCommand::ResolveUserVerification {
                server_name,
                request_id,
                response,
            } => {
                let (decision, content) = match response {
                    crate::app_command::UserVerificationResponse::Accept { proof } => (
                        ruxex_app_server_protocol::McpServerElicitationAction::Accept,
                        Some(
                            serde_json::to_value(proof)
                                .map_err(|_| "Invalid verification proof".to_string())?,
                        ),
                    ),
                    crate::app_command::UserVerificationResponse::Cancel => (
                        ruxex_app_server_protocol::McpServerElicitationAction::Cancel,
                        None,
                    ),
                };
                AppCommand::ResolveElicitation {
                    server_name,
                    request_id,
                    decision,
                    content,
                    meta: None,
                }
            }
            op => op,
        };
        if let AppCommand::ResolveElicitation {
            server_name,
            request_id,
            ..
        } = &op
        {
            self.user_verification.remove(server_name, request_id);
        }
        let resolution = match &op {
            AppCommand::ExecApproval { id, decision, .. } => self
                .exec_approvals
                .remove(&(thread_id, id.clone()))
                .map(|request_id| {
                    Ok::<AppServerRequestResolution, String>(AppServerRequestResolution {
                        request_id,
                        result: serde_json::to_value(CommandExecutionRequestApprovalResponse {
                            decision: decision.clone(),
                        })
                        .map_err(|err| {
                            format!(
                                "failed to serialize command execution approval response: {err}"
                            )
                        })?,
                    })
                })
                .transpose()?,
            AppCommand::PatchApproval { id, decision } => self
                .file_change_approvals
                .remove(&(thread_id, id.clone()))
                .map(|request_id| {
                    Ok::<AppServerRequestResolution, String>(AppServerRequestResolution {
                        request_id,
                        result: serde_json::to_value(FileChangeRequestApprovalResponse {
                            decision: decision.clone(),
                        })
                        .map_err(|err| {
                            format!("failed to serialize file change approval response: {err}")
                        })?,
                    })
                })
                .transpose()?,
            AppCommand::RequestPermissionsResponse { id, response } => self
                .permissions_approvals
                .remove(&(thread_id, id.clone()))
                .map(|request_id| {
                    Ok::<AppServerRequestResolution, String>(AppServerRequestResolution {
                        request_id,
                        result: serde_json::to_value(PermissionsRequestApprovalResponse {
                            permissions: granted_permission_profile_from_request(
                                response.permissions.clone(),
                            ),
                            scope: response.scope.into(),
                        })
                        .map_err(|err| {
                            format!("failed to serialize permissions approval response: {err}")
                        })?,
                    })
                })
                .transpose()?,
            AppCommand::UserInputAnswer { id, response } => self
                .pop_user_input_request_for_turn(id)
                .map(|pending| {
                    Ok::<AppServerRequestResolution, String>(AppServerRequestResolution {
                        request_id: pending.request_id,
                        result: serde_json::to_value(response).map_err(|err| {
                            format!("failed to serialize request_user_input response: {err}")
                        })?,
                    })
                })
                .transpose()?,
            AppCommand::ResolveElicitation {
                server_name,
                request_id,
                decision,
                content,
                meta,
            } => self
                .mcp_requests
                .remove(&McpRequestKey {
                    server_name: server_name.to_string(),
                    request_id: request_id.clone(),
                })
                .map(|request_id| {
                    Ok::<AppServerRequestResolution, String>(AppServerRequestResolution {
                        request_id,
                        result: serde_json::to_value(McpServerElicitationRequestResponse {
                            action: *decision,
                            content: content.clone(),
                            meta: meta.clone(),
                        })
                        .map_err(|err| {
                            format!("failed to serialize MCP elicitation response: {err}")
                        })?,
                    })
                })
                .transpose()?,
            _ => None,
        };
        Ok(resolution)
    }

    pub(super) fn resolve_notification(
        &mut self,
        thread_id: &str,
        request_id: &AppServerRequestId,
    ) -> Option<ResolvedAppServerRequest> {
        let thread_id = Self::canonical_thread_id(thread_id);
        if let Some(key) = self.exec_approvals.iter().find_map(|(key, value)| {
            (key.0 == thread_id && value == request_id).then(|| key.clone())
        }) {
            self.exec_approvals.remove(&key);
            return Some(ResolvedAppServerRequest::ExecApproval {
                thread_id: key.0,
                id: key.1,
            });
        }

        if let Some(key) = self.file_change_approvals.iter().find_map(|(key, value)| {
            (key.0 == thread_id && value == request_id).then(|| key.clone())
        }) {
            self.file_change_approvals.remove(&key);
            return Some(ResolvedAppServerRequest::FileChangeApproval {
                thread_id: key.0,
                id: key.1,
            });
        }

        if let Some(key) = self.permissions_approvals.iter().find_map(|(key, value)| {
            (key.0 == thread_id && value == request_id).then(|| key.clone())
        }) {
            self.permissions_approvals.remove(&key);
            return Some(ResolvedAppServerRequest::PermissionsApproval {
                thread_id: key.0,
                id: key.1,
            });
        }

        if let Some(pending) = self.remove_user_input_request(request_id) {
            return Some(ResolvedAppServerRequest::UserInput {
                call_id: pending.item_id,
            });
        }

        if let Some(key) = self
            .mcp_requests
            .iter()
            .find_map(|(key, value)| (value == request_id).then(|| key.clone()))
        {
            self.mcp_requests.remove(&key);
            self.user_verification
                .remove(&key.server_name, &key.request_id);
            return Some(ResolvedAppServerRequest::McpElicitation {
                server_name: key.server_name,
                request_id: key.request_id,
            });
        }

        None
    }

    pub(super) fn contains_server_request(&self, request: &ServerRequest) -> bool {
        match request {
            ServerRequest::CommandExecutionRequestApproval { request_id, .. } => self
                .exec_approvals
                .values()
                .any(|pending_request_id| pending_request_id == request_id),
            ServerRequest::FileChangeRequestApproval { request_id, .. } => self
                .file_change_approvals
                .values()
                .any(|pending_request_id| pending_request_id == request_id),
            ServerRequest::PermissionsRequestApproval { request_id, .. } => self
                .permissions_approvals
                .values()
                .any(|pending_request_id| pending_request_id == request_id),
            ServerRequest::ToolRequestUserInput { request_id, .. } => {
                self.user_inputs.values().any(|queue| {
                    queue
                        .iter()
                        .any(|pending| &pending.request_id == request_id)
                })
            }
            ServerRequest::McpServerElicitationRequest { request_id, .. } => self
                .mcp_requests
                .values()
                .any(|pending_request_id| pending_request_id == request_id),
            ServerRequest::DynamicToolCall { .. }
            | ServerRequest::AttestationGenerate { .. }
            | ServerRequest::CurrentTimeRead { .. } => false,
        }
    }

    fn pop_user_input_request_for_turn(
        &mut self,
        turn_id: &str,
    ) -> Option<PendingUserInputRequest> {
        let pending = self
            .user_inputs
            .get_mut(turn_id)
            .and_then(VecDeque::pop_front);
        if self
            .user_inputs
            .get(turn_id)
            .is_some_and(VecDeque::is_empty)
        {
            self.user_inputs.remove(turn_id);
        }
        pending
    }

    fn remove_user_input_request(
        &mut self,
        request_id: &AppServerRequestId,
    ) -> Option<PendingUserInputRequest> {
        let (turn_id, index) = self.user_inputs.iter().find_map(|(turn_id, queue)| {
            queue
                .iter()
                .position(|pending| &pending.request_id == request_id)
                .map(|index| (turn_id.clone(), index))
        })?;
        let queue = self.user_inputs.get_mut(&turn_id)?;
        let removed = queue.remove(index);
        if queue.is_empty() {
            self.user_inputs.remove(&turn_id);
        }
        removed
    }
}

#[derive(Debug)]
struct PendingUserInputRequest {
    item_id: String,
    request_id: AppServerRequestId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct McpRequestKey {
    server_name: String,
    request_id: AppServerRequestId,
}
