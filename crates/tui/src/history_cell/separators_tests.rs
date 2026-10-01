//! Completion metadata rendering, date selection, and muted styling coverage.

use super::*;
use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::Timelike;
use codex_otel::RuntimeMetricTotals;
use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn completed_at() -> DateTime<Local> {
    let datetime = NaiveDateTime::parse_from_str("2000-09-06 14:32:00", "%Y-%m-%d %H:%M:%S")
        .expect("valid completion time");
    Local
        .from_local_datetime(&datetime)
        .single()
        .expect("unambiguous completion time")
}




#[test]
fn completion_without_metadata_has_no_visible_or_raw_lines() {
    let cell =
        FinalMessageSeparator::new(/*elapsed_seconds*/ None, /*runtime_metrics*/ None);
    assert_eq!(cell.display_lines(/*width*/ 80), Vec::<Line>::new());
    assert_eq!(cell.raw_lines(), Vec::<Line>::new());
}



#[test]
fn completion_renders_with_dim_default_colors() {
    let cell: Box<dyn HistoryCell> = Box::new(
        FinalMessageSeparator::new(/*elapsed_seconds*/ None, /*runtime_metrics*/ None)
            .with_completed_at(completed_at()),
    );
    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 80, /*height*/ 1,
    );
    let mut buffer = Buffer::empty(area);
    cell.render(area, &mut buffer);
    let text_width = cell.display_lines(area.width)[0].width();
    let styles = (0..text_width)
        .map(|x| {
            let rendered = &buffer[(x as u16, 0)];
            (rendered.fg, rendered.bg, rendered.modifier)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        styles,
        vec![(Color::Reset, Color::Reset, Modifier::DIM); text_width]
    );
}
