//! Displays local daemon status and the provider-neutral lifecycle boundary.

use super::*;
use crate::bottom_pane::SelectionViewParams;
use crate::wrapping::word_wrap_lines;
use ratatui::buffer::Buffer;
use ratatui::widgets::Paragraph;

struct DaemonMenuHeader(Vec<Line<'static>>);

impl Renderable for DaemonMenuHeader {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        Renderable::render(
            &Paragraph::new(word_wrap_lines(&self.0, usize::from(area.width))),
            area,
            buf,
        );
    }

    fn desired_height(&self, width: u16) -> u16 {
        word_wrap_lines(&self.0, usize::from(width)).len() as u16
    }
}

impl App {
    pub(super) fn open_daemon_menu(&mut self) {
        let status = self
            .chat_widget
            .remote_connection
            .as_ref()
            .filter(|_| matches!(self.app_server_target, AppServerTarget::LocalDaemon { .. }))
            .map(|connection| format!("Running daemon: {}", connection.version))
            .unwrap_or_else(|| "Not connected to the local background server.".to_string());
        let mut header = vec![Line::from("Daemon".bold()), Line::from(status.dim())];
        header.push(Line::from(
            "Daemon package updates are not provided by Ruxex.".dim(),
        ));
        self.chat_widget.show_selection_view(SelectionViewParams {
            header: Box::new(DaemonMenuHeader(header)),
            items: Vec::new(),
            ..Default::default()
        });
    }
}
