//! Authentication step UI and state transitions used by onboarding.
//!
//! This module owns the auth-step state machine for API-key login,
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
use ruxex_app_server_protocol::ModelProviderAuthField;
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
    ApiKeyEntry(ApiKeyInputState),
    ResolverLoading,
    ResolverStep(ResolverInputState),
    ApiKeyConfigured,
}

const API_KEY_DISABLED_MESSAGE: &str = "API key login is disabled.";
pub(super) fn onboarding_request_id() -> ruxex_app_server_protocol::RequestId {
    ruxex_app_server_protocol::RequestId::String(uuid::Uuid::new_v4().to_string())
}

#[derive(Clone, Default)]
pub(crate) struct ApiKeyInputState {
    fields: Vec<ModelProviderAuthField>,
    field_index: usize,
    values: BTreeMap<String, String>,
    value: String,
    prepopulated_from_env: bool,
}

impl ApiKeyInputState {
    fn new(mut fields: Vec<ModelProviderAuthField>) -> Self {
        if fields.is_empty() {
            fields.push(ModelProviderAuthField {
                id: "api_key".to_string(),
                label: "API key".to_string(),
                secret: true,
            });
        }
        Self {
            fields,
            field_index: 0,
            values: BTreeMap::new(),
            value: String::new(),
            prepopulated_from_env: false,
        }
    }

    fn current_field(&self) -> &ModelProviderAuthField {
        &self.fields[self.field_index]
    }

    fn into_credentials(mut self) -> Option<(String, BTreeMap<String, String>)> {
        let current = self.current_field().id.clone();
        self.values.insert(current, self.value.trim().to_string());
        let api_key = self.values.remove("api_key")?;
        if api_key.is_empty() {
            return None;
        }
        Some((api_key, self.values))
    }

    fn next_field(&mut self) -> bool {
        self.values.insert(
            self.current_field().id.clone(),
            self.value.trim().to_string(),
        );
        if self.field_index + 1 >= self.fields.len() {
            return false;
        }
        self.field_index += 1;
        self.value = self
            .values
            .get(self.current_field().id.as_str())
            .cloned()
            .unwrap_or_default();
        self.prepopulated_from_env = false;
        true
    }
}

struct ResolverInputState {
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
        if self.handle_resolver_key_event(&key_event) {
            return;
        }
        if self.handle_api_key_entry_key_event(&key_event) {
            return;
        }
    }

    fn handle_paste(&mut self, pasted: String) {
        let sign_in_state = self.sign_in_state.read().unwrap();
        match &*sign_in_state {
            SignInState::ApiKeyEntry(_) => {
                drop(sign_in_state);
                let _ = self.handle_api_key_entry_paste(pasted);
            }
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
        auth_fields: Vec<ModelProviderAuthField>,
        provider_credential_resolver: Option<SharedProviderCredentialResolver>,
    ) -> Self {
        let auth_fields = if provider_credential_resolver.is_some() {
            auth_fields.into_iter().take(1).collect()
        } else {
            auth_fields
        };
        Self {
            request_frame,
            error: Arc::new(RwLock::new(None)),
            sign_in_state: Arc::new(RwLock::new(SignInState::ApiKeyEntry(
                ApiKeyInputState::new(auth_fields),
            ))),
            app_server_request_handle,
            auth_config,
            provider_credential_resolver,
        }
    }

    pub(crate) fn cancel_active_attempt(&self) {
        let fields = self
            .sign_in_state
            .read()
            .ok()
            .and_then(|state| match &*state {
                SignInState::ApiKeyEntry(state) => Some(state.fields.clone()),
                SignInState::ResolverLoading
                | SignInState::ResolverStep(_)
                | SignInState::ApiKeyConfigured => None,
            })
            .unwrap_or_default();
        *self.sign_in_state.write().unwrap() =
            SignInState::ApiKeyEntry(ApiKeyInputState::new(fields));
        self.set_error(/*message*/ None);
        self.request_frame.schedule_frame();
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
            SignInState::ApiKeyEntry(_) | SignInState::ResolverStep(_) => true,
            _ => false,
        })
    }

    /// Returns whether printable quit shortcuts must be treated as text input.
    ///
    /// provider API-key entry keeps its existing empty-field quit behavior.
    pub(crate) fn should_suppress_printable_quit(&self) -> bool {
        self.sign_in_state.read().is_ok_and(|guard| match &*guard {
            SignInState::ApiKeyEntry(state) => !state.value.is_empty(),
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

    fn disallow_api_login(&mut self) {
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

    fn render_api_key_entry(&self, area: Rect, buf: &mut Buffer, state: &ApiKeyInputState) {
        let [intro_area, input_area, footer_area] = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .areas(area);

        let field = state.current_field();
        let mut intro_lines: Vec<Line> = vec![
            Line::from(vec!["> ".into(), format!("Enter {}", field.label).bold()]),
            "".into(),
            "  This value will be stored locally in auth.json.".into(),
            "".into(),
        ];
        if state.prepopulated_from_env {
            intro_lines.push("  Detected provider API key environment variable.".into());
            intro_lines.push(
                "  Paste a different key if you prefer to use another account."
                    .dim()
                    .into(),
            );
            intro_lines.push("".into());
        }
        Paragraph::new(intro_lines)
            .wrap(Wrap { trim: false })
            .render(intro_area, buf);

        let content_line: Line = if state.value.is_empty() {
            vec![format!("Paste or type your {}", field.label).dim()].into()
        } else {
            Line::from(state.value.clone())
        };
        Paragraph::new(content_line)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(field.label.clone())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .render(input_area, buf);

        let mut footer_lines: Vec<Line> = vec![
            Line::from(vec![
                "  Press ".dim(),
                keys::CONFIRM[0].into(),
                if state.field_index + 1 < state.fields.len() {
                    " to continue".dim()
                } else {
                    " to save".dim()
                },
            ]),
            Line::from(vec![
                "  Press ".dim(),
                self.cancel_binding().into(),
                " to go back".dim(),
            ]),
        ];
        if let Some(error) = self.error_message() {
            footer_lines.push("".into());
            footer_lines.push(error.red().into());
        }
        Paragraph::new(footer_lines)
            .wrap(Wrap { trim: false })
            .render(footer_area, buf);
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

    fn handle_api_key_entry_key_event(&mut self, key_event: &KeyEvent) -> bool {
        let mut should_save: Option<String> = None;
        let mut credentials = BTreeMap::new();
        let mut retry_state = None;
        let mut should_request_frame = false;

        {
            let mut guard = self.sign_in_state.write().unwrap();
            if let SignInState::ApiKeyEntry(state) = &mut *guard {
                if keys::CANCEL.is_pressed(*key_event) {
                    let fields = state.fields.clone();
                    *guard = SignInState::ApiKeyEntry(ApiKeyInputState::new(fields));
                    self.set_error(/*message*/ None);
                    should_request_frame = true;
                } else if keys::CONFIRM.is_pressed(*key_event) {
                    let trimmed = state.value.trim().to_string();
                    if trimmed.is_empty() {
                        let label = state.current_field().label.clone();
                        self.set_error(Some(format!("{label} cannot be empty")));
                        should_request_frame = true;
                    } else if state.field_index + 1 < state.fields.len() {
                        state.next_field();
                        self.set_error(/*message*/ None);
                        should_request_frame = true;
                    } else {
                        retry_state = Some(state.clone());
                        let completed = state.clone();
                        if let Some((api_key, extra_credentials)) = completed.into_credentials() {
                            should_save = Some(api_key);
                            credentials = extra_credentials;
                        }
                    }
                } else {
                    match key_event.code {
                        KeyCode::Backspace => {
                            if state.prepopulated_from_env {
                                state.value.clear();
                                state.prepopulated_from_env = false;
                            } else {
                                state.value.pop();
                            }
                            self.set_error(/*message*/ None);
                            should_request_frame = true;
                        }
                        KeyCode::Char(c)
                            if key_event.kind == KeyEventKind::Press
                                && !key_event.modifiers.contains(KeyModifiers::SUPER)
                                && !key_event.modifiers.contains(KeyModifiers::CONTROL)
                                && !key_event.modifiers.contains(KeyModifiers::ALT) =>
                        {
                            if state.prepopulated_from_env {
                                state.value.clear();
                                state.prepopulated_from_env = false;
                            }
                            state.value.push(c);
                            self.set_error(/*message*/ None);
                            should_request_frame = true;
                        }
                        _ => {}
                    }
                }
                // handled; let guard drop before potential save
            } else {
                return false;
            }
        }

        if let Some(api_key) = should_save {
            if self.provider_credential_resolver.is_some() {
                self.start_resolver(api_key);
            } else {
                self.save_api_key(api_key, credentials, retry_state.expect("save state"));
            }
        } else if should_request_frame {
            self.request_frame.schedule_frame();
        }
        true
    }

    fn start_resolver(&mut self, api_key: String) {
        if !self.is_api_login_allowed() {
            self.disallow_api_login();
            return;
        }
        let Some(resolver) = self.provider_credential_resolver.clone() else {
            return;
        };
        *self.sign_in_state.write().unwrap() = SignInState::ResolverLoading;
        self.set_error(None);
        let sign_in_state = self.sign_in_state.clone();
        let error = self.error.clone();
        let request_frame = self.request_frame.clone();
        tokio::spawn(async move {
            let retry_api_key = api_key.clone();
            let result = async {
                let mut flow = resolver.begin(CredentialInput::empty()).await?;
                let step = flow.submit(CredentialValue::secret(api_key)).await?;
                Ok::<_, ruxex_model_provider::ProviderError>((flow, step))
            }
            .await;
            match result {
                Ok((flow, step)) => {
                    *error.write().unwrap() = None;
                    *sign_in_state.write().unwrap() =
                        SignInState::ResolverStep(ResolverInputState::new(flow, step));
                }
                Err(err) => {
                    *error.write().unwrap() =
                        Some(format!("Failed to discover credentials: {err}"));
                    *sign_in_state.write().unwrap() = SignInState::ApiKeyEntry(ApiKeyInputState {
                        fields: vec![ModelProviderAuthField {
                            id: "api_key".to_string(),
                            label: "API key".to_string(),
                            secret: true,
                        }],
                        field_index: 0,
                        values: BTreeMap::new(),
                        value: retry_api_key,
                        prepopulated_from_env: true,
                    });
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
        let mut should_request_frame = false;
        {
            let mut guard = self.sign_in_state.write().unwrap();
            let SignInState::ResolverStep(state) = &mut *guard else {
                return false;
            };
            match &state.step {
                CredentialStep::Select { options, .. } => {
                    if keys::CANCEL.is_pressed(*key_event) {
                        *guard = SignInState::ApiKeyEntry(ApiKeyInputState::new(vec![]));
                        should_request_frame = true;
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
                        *guard = SignInState::ApiKeyEntry(ApiKeyInputState::new(vec![]));
                        should_request_frame = true;
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
            }
        }
        if let Some(value) = submit {
            self.submit_resolver_value(value);
        } else if finish {
            match self.finish_resolver() {
                Some(Ok((api_key, credentials))) => {
                    self.save_api_key(api_key, credentials, ApiKeyInputState::new(vec![]));
                }
                Some(Err(message)) => {
                    *self.sign_in_state.write().unwrap() =
                        SignInState::ApiKeyEntry(ApiKeyInputState::new(vec![]));
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
        tokio::spawn(async move {
            let mut flow = flow;
            let result = flow.submit(value).await.map(|step| (flow, step));
            match result {
                Ok((flow, CredentialStep::Complete)) => {
                    *sign_in_state.write().unwrap() = SignInState::ResolverStep(
                        ResolverInputState::new(flow, CredentialStep::Complete),
                    );
                }
                Ok((flow, step)) => {
                    *error.write().unwrap() = None;
                    *sign_in_state.write().unwrap() =
                        SignInState::ResolverStep(ResolverInputState::new(flow, step));
                }
                Err(err) => {
                    *error.write().unwrap() = Some(format!("Failed to resolve credentials: {err}"));
                    *sign_in_state.write().unwrap() =
                        SignInState::ApiKeyEntry(ApiKeyInputState::new(vec![]));
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
        Some(flow.finish().map_or_else(
            |err| Err(format!("Failed to complete credentials: {err}")),
            |mut credentials| {
                credentials
                    .remove("api_key")
                    .map(|api_key| Ok((api_key, credentials)))
                    .unwrap_or_else(|| {
                        Err("Resolved credentials did not contain an API key".to_string())
                    })
            },
        ))
    }

    fn handle_api_key_entry_paste(&mut self, pasted: String) -> bool {
        let trimmed = pasted.trim();
        if trimmed.is_empty() {
            return false;
        }

        let mut guard = self.sign_in_state.write().unwrap();
        if let SignInState::ApiKeyEntry(state) = &mut *guard {
            if state.prepopulated_from_env {
                state.value = trimmed.to_string();
                state.prepopulated_from_env = false;
            } else {
                state.value.push_str(trimmed);
            }
            self.set_error(/*message*/ None);
        } else {
            return false;
        }

        drop(guard);
        self.request_frame.schedule_frame();
        true
    }

    fn save_api_key(
        &mut self,
        api_key: String,
        credentials: BTreeMap<String, String>,
        retry_state: ApiKeyInputState,
    ) {
        if !self.is_api_login_allowed() {
            self.disallow_api_login();
            return;
        }
        self.set_error(/*message*/ None);
        let request_handle = self.app_server_request_handle.clone();
        let sign_in_state = self.sign_in_state.clone();
        let error = self.error.clone();
        let request_frame = self.request_frame.clone();
        tokio::spawn(async move {
            match request_handle
                .request_typed::<LoginAccountResponse>(ClientRequest::LoginAccount {
                    request_id: onboarding_request_id(),
                    params: LoginAccountParams::ApiKey {
                        api_key: api_key.clone(),
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
                    *sign_in_state.write().unwrap() = SignInState::ApiKeyEntry(retry_state);
                }
            }
            request_frame.schedule_frame();
        });
        self.request_frame.schedule_frame();
    }
}

impl StepStateProvider for AuthModeWidget {
    fn get_step_state(&self) -> StepState {
        let sign_in_state = self.sign_in_state.read().unwrap();
        match &*sign_in_state {
            SignInState::ApiKeyEntry(_) => StepState::InProgress,
            SignInState::ResolverLoading | SignInState::ResolverStep(_) => StepState::InProgress,
            SignInState::ApiKeyConfigured => StepState::Complete,
        }
    }
}

impl WidgetRef for AuthModeWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let sign_in_state = self.sign_in_state.read().unwrap();
        match &*sign_in_state {
            SignInState::ApiKeyEntry(state) => {
                self.render_api_key_entry(area, buf, state);
            }
            SignInState::ResolverLoading => self.render_resolver_loading(area, buf),
            SignInState::ResolverStep(state) => self.render_resolver_step(area, buf, state),
            SignInState::ApiKeyConfigured => {
                self.render_api_key_configured(area, buf);
            }
        }
    }
}
