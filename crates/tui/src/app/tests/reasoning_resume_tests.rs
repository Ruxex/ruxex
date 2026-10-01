//! Resume snapshots must accept an unfinished reasoning stream without an earlier item/started.

use super::*;
use crate::chatwidget::tests::helpers::render_bottom_popup;
use genex_app_server_protocol::ItemCompletedNotification;
use genex_app_server_protocol::ReasoningSummaryTextDeltaNotification;



fn reasoning_transcript(events: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>) -> String {
    std::iter::from_fn(|| events.try_recv().ok())
        .filter_map(|event| match event {
            AppEvent::InsertHistoryCell(cell)
                if cell
                    .as_any()
                    .is::<crate::history_cell::ReasoningSummaryCell>() =>
            {
                Some(lines_to_single_string(&cell.transcript_lines(/*width*/ 80)))
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
