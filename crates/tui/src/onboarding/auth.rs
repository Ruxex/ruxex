//! Authentication step UI and state transitions used by onboarding.
//!
//! This module owns the client-side provider credential resolver state machine,
//! renders the corresponding UI, and handles auth-scoped keyboard input.
//! It intentionally does not decide onboarding flow completion; the enclosing
//! onboarding screen coordinates step progression.

#![allow(clippy::unwrap_used)]

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;
use ruxex_app_server_client::AppServerRequestHandle;
use ruxex_app_server_protocol::ClientRequest;
use ruxex_app_server_protocol::LoginAccountParams;
use ruxex_app_server_protocol::LoginAccountResponse;
use ruxex_login::AuthConfig;
use ruxex_provider_auth::CredentialInput;
use ruxex_provider_auth::CredentialStep;
use ruxex_provider_auth::CredentialValue;
use ruxex_provider_auth::ErasedCredentialFlow;
use ruxex_provider_auth::SharedProviderCredentialResolver;

use ruxex_protocol::config_types::ForcedLoginMethod;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;

use crate::key_hint::KeyBinding;
use crate::key_hint::KeyBindingListExt;
use crate::onboarding::keys;
use crate::onboarding::onboarding_screen::KeyboardHandler;
use crate::onboarding::onboarding_screen::StepStateProvider;
use crate::tui::FrameRequester;

use super::onboarding_screen::StepState;

/// Marks any underlined buffer cells as an OSC 8 hyperlink.
pub(crate) fn mark_underlined_hyperlink(buf: &mut Buffer, area: Rect, url: &str) {
    crate::terminal_hyperlinks::mark_underlined_hyperlink(buf, area, url);
}

pub(crate) enum SignInState {
    ResolverUnavailable,
    ResolverLoading,
    ResolverStep(ResolverInputState),
    ApiKeyConfigured,
}

const API_KEY_DISABLED_MESSAGE: &str = "API key login is disabled.";
pub(super) fn onboarding_request_id() -> ruxex_app_server_protocol::RequestId {
    ruxex_app_server_protocol::RequestId::String(uuid::Uuid::new_v4().to_string())
}

pub(crate) struct ResolverInputState {
    flow: Option<Box<dyn ErasedCredentialFlow>>,
    step: CredentialStep,
    value: String,
    selected: usize,
}

impl ResolverInputState {
    fn new(flow: Box<dyn ErasedCredentialFlow>, step: CredentialStep) -> Self {
        Self {
            flow: Some(flow),
            step,
            value: String::new(),
            selected: 0,
        }
    }

    fn selected_value(&self) -> Option<String> {
        let CredentialStep::Select { options, .. } = &self.step else {
            return None;
        };
        options
            .get(self.selected)
            .map(|option| option.value.clone())
    }
}

impl KeyboardHandler for AuthModeWidget {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        let _ = self.handle_resolver_key_event(&key_event);
    }

    fn handle_paste(&mut self, pasted: String) {
        let sign_in_state = self.sign_in_state.read().unwrap();
        match &*sign_in_state {
            SignInState::ResolverStep(state)
                if matches!(state.step, CredentialStep::Input { .. }) =>
            {
                drop(sign_in_state);
                let _ = self.handle_resolver_input_paste(pasted);
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct AuthModeWidget {
    pub request_frame: FrameRequester,
    pub error: Arc<RwLock<Option<String>>>,
    pub sign_in_state: Arc<RwLock<SignInState>>,
    pub app_server_request_handle: AppServerRequestHandle,
    pub auth_config: AuthConfig,
    pub provider_credential_resolver: Option<SharedProviderCredentialResolver>,
}

impl AuthModeWidget {
    pub(crate) fn new(
        request_frame: FrameRequester,
        app_server_request_handle: AppServerRequestHandle,
        auth_config: AuthConfig,
        provider_credential_resolver: Option<SharedProviderCredentialResolver>,
    ) -> Self {
        let sign_in_state = if provider_credential_resolver.is_some() {
            SignInState::ResolverLoading
        } else {
            SignInState::ResolverUnavailable
        };
        let widget = Self {
            request_frame,
            error: Arc::new(RwLock::new(None)),
            sign_in_state: Arc::new(RwLock::new(sign_in_state)),
            app_server_request_handle,
            auth_config,
            provider_credential_resolver,
        };
        if widget.provider_credential_resolver.is_some() {
            widget.begin_resolver();
        }
        widget
    }

    pub(crate) fn cancel_active_attempt(&self) {
        self.begin_resolver();
    }

    fn set_error(&self, message: Option<String>) {
        *self.error.write().unwrap() = message;
    }

    fn error_message(&self) -> Option<String> {
        self.error.read().unwrap().clone()
    }

    /// Returns whether the auth flow is currently accepting text input.
    pub(crate) fn is_text_entry_active(&self) -> bool {
        self.sign_in_state.read().is_ok_and(|guard| match &*guard {
            SignInState::ResolverStep(_) => true,
            _ => false,
        })
    }

    /// Returns whether printable quit shortcuts must be treated as text input.
    ///
    /// Resolver input keeps its existing empty-field quit behavior.
    pub(crate) fn should_suppress_printable_quit(&self) -> bool {
        self.sign_in_state.read().is_ok_and(|guard| match &*guard {
            SignInState::ResolverStep(state) => !state.value.is_empty(),
            _ => false,
        })
    }

    fn cancel_binding(&self) -> KeyBinding {
        keys::CANCEL[0]
    }

    fn is_api_login_allowed(&self) -> bool {
        self.auth_config
            .is_login_method_allowed(ForcedLoginMethod::Api)
    }

    fn disallow_api_login(&self) {
        *self.sign_in_state.write().unwrap() = SignInState::ResolverUnavailable;
        self.set_error(Some(API_KEY_DISABLED_MESSAGE.to_string()));
        self.request_frame.schedule_frame();
    }

    fn render_api_key_configured(&self, area: Rect, buf: &mut Buffer) {
        let lines = vec![
            "✓ API key configured".fg(Color::Green).into(),
            "".into(),
            "  Ruxex will use usage-based billing with your API key.".into(),
        ];

        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn render_resolver_unavailable(&self, area: Rect, buf: &mut Buffer) {
        let mut lines = vec![
            Line::from("> Provider credentials are unavailable.".bold()),
            "".into(),
        ];
        if self.provider_credential_resolver.is_some() {
            lines.push("  The client-side credential flow could not be completed.".into());
            lines.push("  Press Enter to retry.".into());
        } else {
            lines.push(
                "  A client-side credential resolver is not registered for this provider.".into(),
            );
            lines.push("  Press Enter to retry after registering the resolver.".into());
        }
        if let Some(error) = self.error_message() {
            lines.push("".into());
            lines.push(error.red().into());
        }
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn render_resolver_loading(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(vec![
            Line::from("> Discovering provider credentials...".bold()),
            "".into(),
            "  Please wait while the provider account options are loaded.".into(),
        ])
        .wrap(Wrap { trim: false })
        .render(area, buf);
    }

    fn render_resolver_step(&self, area: Rect, buf: &mut Buffer, state: &ResolverInputState) {
        let [intro_area, input_area, footer_area] = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(5),
            Constraint::Min(2),
        ])
        .areas(area);
        let mut footer_lines = vec![
            Line::from(vec![
                "  Press ".dim(),
                keys::CONFIRM[0].into(),
                " to continue".dim(),
            ]),
            Line::from(vec![
                "  Press ".dim(),
                self.cancel_binding().into(),
                " to go back".dim(),
            ]),
        ];
        match &state.step {
            CredentialStep::Input { label, secret, .. } => {
                Paragraph::new(vec![
                    Line::from(vec!["> ".into(), format!("Enter {label}").bold()]),
                    "".into(),
                    "  This value stays on this client until all credentials are complete.".into(),
                ])
                .wrap(Wrap { trim: false })
                .render(intro_area, buf);
                let displayed: Line = if state.value.is_empty() {
                    format!("Enter {label}").dim().into()
                } else if *secret {
                    "•".repeat(state.value.chars().count()).into()
                } else {
                    state.value.clone().into()
                };
                Paragraph::new(displayed)
                    .block(
                        Block::default()
                            .title(label.clone())
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::Cyan)),
                    )
                    .render(input_area, buf);
            }
            CredentialStep::Select { label, options, .. } => {
                Paragraph::new(vec![
                    Line::from(vec!["> ".into(), label.clone().bold()]),
                    "".into(),
                    "  Use ↑/↓ to choose an option.".into(),
                ])
                .wrap(Wrap { trim: false })
                .render(intro_area, buf);
                let lines = options
                    .iter()
                    .enumerate()
                    .map(|(index, option)| {
                        if index == state.selected {
                            Line::from(format!("› {}", option.label)).fg(Color::Cyan)
                        } else {
                            Line::from(format!("  {}", option.label))
                        }
                    })
                    .collect::<Vec<_>>();
                Paragraph::new(lines)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded),
                    )
                    .render(input_area, buf);
            }
            CredentialStep::Info { message } => {
                Paragraph::new(vec![
                    Line::from("> Provider information".bold()),
                    "".into(),
                    message.clone().into(),
                ])
                .wrap(Wrap { trim: false })
                .render(intro_area, buf);
                footer_lines[0] = Line::from(vec![
                    "  Press ".dim(),
                    keys::CONFIRM[0].into(),
                    " to continue".dim(),
                ]);
            }
            CredentialStep::Complete => {
                Paragraph::new("Completing provider credentials...").render(intro_area, buf);
            }
        }
        if let Some(error) = self.error_message() {
            footer_lines.push("".into());
            footer_lines.push(error.red().into());
        }
        Paragraph::new(footer_lines)
            .wrap(Wrap { trim: false })
            .render(footer_area, buf);
    }

    fn begin_resolver(&self) {
        let Some(resolver) = self.provider_credential_resolver.clone() else {
            *self.sign_in_state.write().unwrap() = SignInState::ResolverUnavailable;
            self.set_error(Some(
                "No client-side credential resolver is registered for this provider.".to_string(),
            ));
            self.request_frame.schedule_frame();
            return;
        };
        if !self.is_api_login_allowed() {
            self.disallow_api_login();
            return;
        }
        *self.sign_in_state.write().unwrap() = SignInState::ResolverLoading;
        self.set_error(None);
        let sign_in_state = self.sign_in_state.clone();
        let error = self.error.clone();
        let request_frame = self.request_frame.clone();
        tokio::spawn(async move {
            match resolver.begin(CredentialInput::empty()).await {
                Ok(flow) => {
                    let step = flow.current_step();
                    *error.write().unwrap() = None;
                    *sign_in_state.write().unwrap() =
                        SignInState::ResolverStep(ResolverInputState::new(flow, step));
                }
                Err(err) => {
                    *error.write().unwrap() =
                        Some(format!("Failed to initialize provider credentials: {err}"));
                    *sign_in_state.write().unwrap() = SignInState::ResolverUnavailable;
                }
            }
            request_frame.schedule_frame();
        });
        self.request_frame.schedule_frame();
    }

    fn handle_resolver_input_paste(&mut self, pasted: String) -> bool {
        let trimmed = pasted.trim();
        if trimmed.is_empty() {
            return false;
        }
        let mut guard = self.sign_in_state.write().unwrap();
        let SignInState::ResolverStep(state) = &mut *guard else {
            return false;
        };
        if matches!(state.step, CredentialStep::Input { .. }) {
            state.value.push_str(trimmed);
            self.set_error(None);
            drop(guard);
            self.request_frame.schedule_frame();
            true
        } else {
            false
        }
    }

    fn handle_resolver_key_event(&mut self, key_event: &KeyEvent) -> bool {
        let mut submit = None;
        let mut finish = false;
        let mut restart = false;
        let mut should_request_frame = false;
        {
            let mut guard = self.sign_in_state.write().unwrap();
            match &mut *guard {
                SignInState::ResolverUnavailable => {
                    if keys::CONFIRM.is_pressed(*key_event) {
                        restart = true;
                    }
                }
                SignInState::ResolverStep(state) => match &state.step {
                    CredentialStep::Select { options, .. } => {
                        if keys::CANCEL.is_pressed(*key_event) {
                            restart = true;
                        } else {
                            match key_event.code {
                                KeyCode::Up => {
                                    if state.selected > 0 {
                                        state.selected -= 1;
                                    }
                                    should_request_frame = true;
                                }
                                KeyCode::Down => {
                                    if state.selected + 1 < options.len() {
                                        state.selected += 1;
                                    }
                                    should_request_frame = true;
                                }
                                _ if keys::CONFIRM.is_pressed(*key_event) => {
                                    submit = state.selected_value().map(CredentialValue::selection);
                                }
                                _ => {}
                            }
                        }
                    }
                    CredentialStep::Input { .. } => {
                        if keys::CANCEL.is_pressed(*key_event) {
                            restart = true;
                        } else if keys::CONFIRM.is_pressed(*key_event) {
                            if state.value.trim().is_empty() {
                                self.set_error(Some("Value cannot be empty".to_string()));
                                should_request_frame = true;
                            } else {
                                submit = Some(CredentialValue::text(state.value.trim()));
                            }
                        } else {
                            match key_event.code {
                                KeyCode::Backspace => {
                                    state.value.pop();
                                    self.set_error(None);
                                    should_request_frame = true;
                                }
                                KeyCode::Char(c)
                                    if key_event.kind == KeyEventKind::Press
                                        && !key_event.modifiers.contains(KeyModifiers::SUPER)
                                        && !key_event.modifiers.contains(KeyModifiers::CONTROL)
                                        && !key_event.modifiers.contains(KeyModifiers::ALT) =>
                                {
                                    state.value.push(c);
                                    self.set_error(None);
                                    should_request_frame = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    CredentialStep::Info { .. } => {
                        if keys::CONFIRM.is_pressed(*key_event) {
                            submit = Some(CredentialValue::text(String::new()));
                        }
                    }
                    CredentialStep::Complete => {
                        if keys::CONFIRM.is_pressed(*key_event) {
                            finish = true;
                        }
                    }
                },
                SignInState::ResolverLoading | SignInState::ApiKeyConfigured => {}
            }
        }
        if restart {
            self.begin_resolver();
        } else if let Some(value) = submit {
            self.submit_resolver_value(value);
        } else if finish {
            match self.finish_resolver() {
                Some(Ok((api_key, credentials))) => {
                    self.save_api_key(api_key, credentials);
                }
                Some(Err(message)) => {
                    *self.sign_in_state.write().unwrap() = SignInState::ResolverUnavailable;
                    self.set_error(Some(message));
                    self.request_frame.schedule_frame();
                }
                None => {}
            }
        } else if should_request_frame {
            self.request_frame.schedule_frame();
        }
        true
    }

    fn submit_resolver_value(&mut self, value: CredentialValue) {
        let flow = {
            let mut guard = self.sign_in_state.write().unwrap();
            let SignInState::ResolverStep(state) = &mut *guard else {
                return;
            };
            let Some(flow) = state.flow.take() else {
                return;
            };
            *guard = SignInState::ResolverLoading;
            flow
        };
        let sign_in_state = self.sign_in_state.clone();
        let error = self.error.clone();
        let request_frame = self.request_frame.clone();
        let request_handle = self.app_server_request_handle.clone();
        let auth_login_allowed = self.is_api_login_allowed();
        tokio::spawn(async move {
            let mut flow = flow;
            let result = flow.submit(value).await.map(|step| (flow, step));
            match result {
                Ok((flow, CredentialStep::Complete)) => match finish_resolver_flow(flow) {
                    Ok((api_key, credentials)) if auth_login_allowed => {
                        *error.write().unwrap() = None;
                        save_api_key_request(
                            request_handle,
                            sign_in_state,
                            error,
                            request_frame.clone(),
                            api_key,
                            credentials,
                        )
                        .await;
                    }
                    Ok(_) => {
                        *sign_in_state.write().unwrap() = SignInState::ResolverUnavailable;
                        *error.write().unwrap() = Some(API_KEY_DISABLED_MESSAGE.to_string());
                    }
                    Err(message) => {
                        *sign_in_state.write().unwrap() = SignInState::ResolverUnavailable;
                        *error.write().unwrap() = Some(message);
                    }
                },
                Ok((flow, step)) => {
                    *error.write().unwrap() = None;
                    *sign_in_state.write().unwrap() =
                        SignInState::ResolverStep(ResolverInputState::new(flow, step));
                }
                Err(err) => {
                    *error.write().unwrap() = Some(format!("Failed to resolve credentials: {err}"));
                    *sign_in_state.write().unwrap() = SignInState::ResolverUnavailable;
                }
            }
            request_frame.schedule_frame();
        });
        self.request_frame.schedule_frame();
    }

    fn finish_resolver(&self) -> Option<Result<(String, BTreeMap<String, String>), String>> {
        let mut guard = self.sign_in_state.write().unwrap();
        let SignInState::ResolverStep(state) =
            std::mem::replace(&mut *guard, SignInState::ResolverLoading)
        else {
            return None;
        };
        let Some(flow) = state.flow else {
            return None;
        };
        Some(finish_resolver_flow(flow))
    }

    fn save_api_key(&mut self, api_key: String, credentials: BTreeMap<String, String>) {
        if !self.is_api_login_allowed() {
            self.disallow_api_login();
            return;
        }
        self.set_error(/*message*/ None);
        let request_frame = self.request_frame.clone();
        tokio::spawn(save_api_key_request(
            self.app_server_request_handle.clone(),
            self.sign_in_state.clone(),
            self.error.clone(),
            request_frame.clone(),
            api_key,
            credentials,
        ));
        self.request_frame.schedule_frame();
    }
}

fn finish_resolver_flow(
    flow: Box<dyn ErasedCredentialFlow>,
) -> Result<(String, BTreeMap<String, String>), String> {
    flow.finish().map_or_else(
        |err| Err(format!("Failed to complete credentials: {err}")),
        |mut credentials| {
            credentials
                .remove("api_key")
                .map(|api_key| Ok((api_key, credentials)))
                .unwrap_or_else(|| {
                    Err("Resolved credentials did not contain an API key".to_string())
                })
        },
    )
}

async fn save_api_key_request(
    request_handle: AppServerRequestHandle,
    sign_in_state: Arc<RwLock<SignInState>>,
    error: Arc<RwLock<Option<String>>>,
    request_frame: FrameRequester,
    api_key: String,
    credentials: BTreeMap<String, String>,
) {
    match request_handle
        .request_typed::<LoginAccountResponse>(ClientRequest::LoginAccount {
            request_id: onboarding_request_id(),
            params: LoginAccountParams::ApiKey {
                api_key,
                credentials,
            },
        })
        .await
    {
        Ok(LoginAccountResponse::ApiKey {}) => {
            *error.write().unwrap() = None;
            *sign_in_state.write().unwrap() = SignInState::ApiKeyConfigured;
        }
        Err(err) => {
            *error.write().unwrap() = Some(format!("Failed to save API key: {err}"));
            *sign_in_state.write().unwrap() = SignInState::ResolverUnavailable;
        }
    }
    request_frame.schedule_frame();
}

impl StepStateProvider for AuthModeWidget {
    fn get_step_state(&self) -> StepState {
        let sign_in_state = self.sign_in_state.read().unwrap();
        match &*sign_in_state {
            SignInState::ResolverUnavailable
            | SignInState::ResolverLoading
            | SignInState::ResolverStep(_) => StepState::InProgress,
            SignInState::ApiKeyConfigured => StepState::Complete,
        }
    }
}

impl WidgetRef for AuthModeWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let sign_in_state = self.sign_in_state.read().unwrap();
        match &*sign_in_state {
            SignInState::ResolverUnavailable => self.render_resolver_unavailable(area, buf),
            SignInState::ResolverLoading => self.render_resolver_loading(area, buf),
            SignInState::ResolverStep(state) => self.render_resolver_step(area, buf, state),
            SignInState::ApiKeyConfigured => {
                self.render_api_key_configured(area, buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruxex_model_provider::ProviderFuture;
    use ruxex_model_provider::ProviderResult;

    struct CompletedFlow;

    impl ErasedCredentialFlow for CompletedFlow {
        fn current_step(&self) -> CredentialStep {
            CredentialStep::Complete
        }

        fn submit(
            &mut self,
            _value: CredentialValue,
        ) -> ProviderFuture<'_, ProviderResult<CredentialStep>> {
            Box::pin(async { Ok(CredentialStep::Complete) })
        }

        fn finish(self: Box<Self>) -> ProviderResult<BTreeMap<String, String>> {
            Ok(BTreeMap::from([
                ("api_key".to_string(), "secret".to_string()),
                ("account".to_string(), "example".to_string()),
            ]))
        }
    }

    #[tokio::test]
    async fn completed_credential_flow_finishes_without_an_extra_confirmation() {
        let mut flow: Box<dyn ErasedCredentialFlow> = Box::new(CompletedFlow);
        assert_eq!(
            flow.submit(CredentialValue::text(String::new()))
                .await
                .expect("credential flow should complete"),
            CredentialStep::Complete
        );
        let credentials = finish_resolver_flow(flow)
            .expect("completed credential flow should produce credentials");

        assert_eq!(credentials.0, "secret");
        assert_eq!(credentials.1["account"], "example");
    }
}
