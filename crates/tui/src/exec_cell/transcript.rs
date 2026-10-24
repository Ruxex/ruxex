//! Expanded command history, preserving reasoning between grouped exploration calls.

use super::model::ExecCell;
use crate::exec_command::strip_bash_lc_and_escape;
use crate::history_cell::HistoryRenderMode;
use crate::render::highlight::highlight_bash_to_lines;
use crate::render::line_utils::push_owned_lines;
use crate::wrapping::RtOptions;
use crate::wrapping::adaptive_wrap_line;
use crate::wrapping::adaptive_wrap_lines;
use ratatui::prelude::*;
use ruxex_ansi_escape::ansi_escape_line;
use ruxex_utils_elapsed::format_duration;

impl ExecCell {
    pub(super) fn detailed_lines(&self, width: u16, mode: HistoryRenderMode) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let mut reasoning = self.reasoning.iter().peekable();
        for (index, call) in self.calls.iter().enumerate() {
            if index > 0 {
                lines.push("".into());
            }
            let script = strip_bash_lc_and_escape(&call.command);
            let highlighted_script = highlight_bash_to_lines(&script);
            lines.extend(adaptive_wrap_lines(
                &highlighted_script,
                RtOptions::new(width as usize)
                    .initial_indent("$ ".magenta().into())
                    .subsequent_indent("    ".into()),
            ));

            if let Some(output) = call.output.as_ref() {
                if !call.is_unified_exec_interaction() {
                    let wrap_opts = RtOptions::new(width.max(1) as usize);
                    for line in output
                        .transcript_lines()
                        .map(|line| ansi_escape_line(line.as_ref()))
                    {
                        push_owned_lines(&adaptive_wrap_line(&line, wrap_opts.clone()), &mut lines);
                    }
                }
                if let Some(duration) = call.duration {
                    let mut result = if output.exit_code == 0 {
                        Line::from("✓".green().bold())
                    } else {
                        Line::from(vec![
                            "✗".red().bold(),
                            format!(" ({})", output.exit_code).into(),
                        ])
                    };
                    result.push_span(format!(" • {}", format_duration(duration)).dim());
                    lines.push(result);
                }
            }

            if mode == HistoryRenderMode::Rich {
                while let Some((_after_calls, cell)) =
                    reasoning.next_if(|(after_calls, _)| *after_calls == index + 1)
                {
                    let reasoning_lines = cell.transcript_lines(width);
                    if !reasoning_lines.is_empty() {
                        lines.push("".into());
                        lines.extend(reasoning_lines);
                    }
                }
            }
        }
        lines
    }
}
