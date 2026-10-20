use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;

use crate::brand::TuiBrandProfile;
use crate::onboarding::onboarding_screen::KeyboardHandler;
use crate::onboarding::onboarding_screen::StepStateProvider;

use super::onboarding_screen::StepState;

pub(crate) struct WelcomeWidget {
    pub is_logged_in: bool,
    brand_profile: Arc<dyn TuiBrandProfile>,
}

impl KeyboardHandler for WelcomeWidget {
    fn handle_key_event(&mut self, _key_event: KeyEvent) {}
}

impl WelcomeWidget {
    pub(crate) fn new(is_logged_in: bool, brand_profile: Arc<dyn TuiBrandProfile>) -> Self {
        Self {
            is_logged_in,
            brand_profile,
        }
    }
}

impl WidgetRef for &WelcomeWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let mut lines: Vec<Line<'static>> = Vec::new();
        if let Some(art) = self.brand_profile.welcome_art() {
            lines.extend(art.iter().map(|line| Line::from((*line).to_string().dim())));
        }

        let product_name = self.brand_profile.product_name();
        let tagline = self.brand_profile.welcome_tagline();
        let version = self.brand_profile.version_label();
        if !lines.is_empty() && (product_name.is_some() || tagline.is_some() || version.is_some()) {
            lines.push(Line::from(""));
        }
        if let Some(product_name) = product_name {
            lines.push(Line::from(vec![
                "Welcome to ".into(),
                product_name.to_string().bold(),
            ]));
        }
        if let Some(tagline) = tagline {
            lines.push(Line::from(tagline.to_string().dim()));
        }
        if let Some(version) = version {
            lines.push(Line::from(version.dim()));
        }

        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}

impl StepStateProvider for WelcomeWidget {
    fn get_step_state(&self) -> StepState {
        match self.is_logged_in {
            true => StepState::Hidden,
            false => StepState::Complete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn row_containing(buf: &Buffer, needle: &str) -> Option<u16> {
        (0..buf.area.height).find(|&y| {
            let mut row = String::new();
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            row.contains(needle)
        })
    }

    #[test]
    fn welcome_renders_centered_branding_with_version() {
        let widget = WelcomeWidget::new(
            /*is_logged_in*/ false,
            crate::brand::default_tui_brand_profile(),
        );
        let area = Rect::new(0, 0, 60, 8);
        let mut buf = Buffer::empty(area);
        (&widget).render_ref(area, &mut buf);

        assert!(row_containing(&buf, "RUXEX").is_some());
        assert!(row_containing(&buf, crate::version::RUXEX_CLI_VERSION).is_some());

        let wordmark_row = row_containing(&buf, "RUXEX").expect("wordmark row");
        let wordmark_start = (0..area.width)
            .find(|&x| buf[(x, wordmark_row)].symbol() != " ")
            .expect("wordmark should have visible content");
        let wordmark_width = crate::brand::WELCOME_ART[1].len() as u16;
        let expected_start = area.left() + (area.width - wordmark_width) / 2;
        assert_eq!(wordmark_start, expected_start);
    }
}
