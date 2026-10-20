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

#[derive(Clone)]
pub(crate) enum SignInState {
    ApiKeyEntry(ApiKeyInputState),
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

impl KeyboardHandler for AuthModeWidget {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
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
}

impl AuthModeWidget {
    pub(crate) fn new(
        request_frame: FrameRequester,
        app_server_request_handle: AppServerRequestHandle,
        auth_config: AuthConfig,
        auth_fields: Vec<ModelProviderAuthField>,
    ) -> Self {
        Self {
            request_frame,
            error: Arc::new(RwLock::new(None)),
            sign_in_state: Arc::new(RwLock::new(SignInState::ApiKeyEntry(
                ApiKeyInputState::new(auth_fields),
            ))),
            app_server_request_handle,
            auth_config,
        }
    }

    pub(crate) fn cancel_active_attempt(&self) {
        let fields = self
            .sign_in_state
            .read()
            .ok()
            .and_then(|state| match &*state {
                SignInState::ApiKeyEntry(state) => Some(state.fields.clone()),
                SignInState::ApiKeyConfigured => None,
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
            SignInState::ApiKeyEntry(_) => true,
            _ => false,
        })
    }

    /// Returns whether printable quit shortcuts must be treated as text input.
    ///
    /// provider API-key entry keeps its existing empty-field quit behavior.
    pub(crate) fn should_suppress_printable_quit(&self) -> bool {
        self.sign_in_state.read().is_ok_and(|guard| match &*guard {
            SignInState::ApiKeyEntry(state) => !state.value.is_empty(),
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
            self.save_api_key(api_key, credentials, retry_state.expect("save state"));
        } else if should_request_frame {
            self.request_frame.schedule_frame();
        }
        true
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
            SignInState::ApiKeyConfigured => {
                self.render_api_key_configured(area, buf);
            }
        }
    }
}
