use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use genex_app_server_protocol::ModelProviderListEntry;
use genex_app_server_protocol::ProviderAuthStatus;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;

use crate::key_hint::KeyBindingListExt;
use crate::onboarding::keys;
use crate::onboarding::onboarding_screen::KeyboardHandler;
use crate::render::renderable::ColumnRenderable;
use crate::render::renderable::Renderable;
use crate::selection_list::selection_option_row;
use crate::tui::Tui;
use crate::tui::TuiEvent;
use color_eyre::eyre::Result;
use tokio_stream::StreamExt;

pub(crate) struct ModelProviderSelectionWidget {
    providers: Vec<ModelProviderListEntry>,
    highlighted: usize,
    selected: Option<ModelProviderListEntry>,
    canceled: bool,
}

impl ModelProviderSelectionWidget {
    pub(crate) fn new(providers: Vec<ModelProviderListEntry>) -> Self {
        Self {
            providers,
            highlighted: 0,
            selected: None,
            canceled: false,
        }
    }

    pub(crate) fn take_result(&mut self) -> Option<Option<ModelProviderListEntry>> {
        if self.canceled {
            Some(None)
        } else {
            self.selected.take().map(Some)
        }
    }
}

impl KeyboardHandler for ModelProviderSelectionWidget {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.kind != KeyEventKind::Press || self.providers.is_empty() {
            return;
        }

        if keys::MOVE_UP.is_pressed(key_event) {
            self.highlighted = self
                .highlighted
                .checked_sub(1)
                .unwrap_or(self.providers.len() - 1);
        } else if keys::MOVE_DOWN.is_pressed(key_event) {
            self.highlighted = (self.highlighted + 1) % self.providers.len();
        } else if keys::SELECT_FIRST.is_pressed(key_event) {
            self.highlighted = 0;
        } else if keys::SELECT_SECOND.is_pressed(key_event) {
            self.highlighted = 1.min(self.providers.len() - 1);
        } else if keys::SELECT_THIRD.is_pressed(key_event) {
            self.highlighted = 2.min(self.providers.len() - 1);
        } else if keys::SELECT_FOURTH.is_pressed(key_event) {
            self.highlighted = 3.min(self.providers.len() - 1);
        } else if keys::CONFIRM.is_pressed(key_event) {
            self.selected = Some(self.providers[self.highlighted].clone());
        } else if keys::CANCEL.is_pressed(key_event) || keys::QUIT.is_pressed(key_event) {
            self.canceled = true;
        }
    }
}

impl WidgetRef for &ModelProviderSelectionWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let mut column = ColumnRenderable::new();
        column.push(Line::from("Select a model provider".bold()));
        column.push(
            Paragraph::new("Choose the provider to authenticate and use.")
                .wrap(Wrap { trim: false }),
        );
        column.push("");

        for (index, provider) in self.providers.iter().enumerate() {
            let status = match provider.auth_status {
                ProviderAuthStatus::Authenticated => "authenticated",
                ProviderAuthStatus::Invalid => "authentication needs attention",
                ProviderAuthStatus::Unauthenticated => "authentication required",
            };
            column.push(selection_option_row(
                index,
                format!("{} ({status})", provider.name),
                index == self.highlighted,
            ));
        }

        column.push("");
        column.push(Line::from(vec![
            "Press ".dim(),
            keys::CONFIRM[0].into(),
            " to continue; ↑/↓ to choose; esc to quit".dim(),
        ]));
        column.render(area, buf);
    }
}

pub(crate) async fn run_model_provider_selection(
    tui: &mut Tui,
    providers: Vec<ModelProviderListEntry>,
) -> Result<Option<ModelProviderListEntry>> {
    if providers.is_empty() {
        color_eyre::eyre::bail!("modelProvider/list returned no providers for TUI bootstrap");
    }

    let mut widget = ModelProviderSelectionWidget::new(providers);
    tui.draw(u16::MAX, |frame| {
        frame.render_widget_ref(&widget, frame.area());
    })?;
    tui.discard_pending_input_before_interactive_screen()?;

    let mut events = tui.event_stream();
    while let Some(event) = events.next().await {
        match event {
            TuiEvent::Key(key_event) => {
                widget.handle_key_event(key_event);
                if let Some(result) = widget.take_result() {
                    return Ok(result);
                }
                tui.draw(u16::MAX, |frame| {
                    frame.render_widget_ref(&widget, frame.area());
                })?;
            }
            TuiEvent::Draw | TuiEvent::Resume | TuiEvent::Resize(_) | TuiEvent::FocusGained => {
                tui.draw(u16::MAX, |frame| {
                    frame.render_widget_ref(&widget, frame.area());
                })?;
            }
            TuiEvent::FocusLost | TuiEvent::Paste(_) => {}
        }
    }

    Ok(None)
}
