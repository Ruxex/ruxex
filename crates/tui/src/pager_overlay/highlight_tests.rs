//! Backtrack selection preserves unrelated layout caches and the complete rendered viewport.

use super::*;
use crate::history_cell::UserHistoryCell;
use crate::keymap::RuntimeKeymap;
use pretty_assertions::assert_eq;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

#[derive(Debug)]
struct MeasuredCell {
    measurements: AtomicUsize,
}

impl HistoryCell for MeasuredCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        vec!["history".into()]
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        vec!["history".into()]
    }

    fn desired_transcript_height(&self, _width: u16) -> u16 {
        self.measurements.fetch_add(1, Ordering::Relaxed);
        1
    }
}

#[test]
fn moving_highlight_preserves_unaffected_height_caches() {
    let cells: Vec<_> = (0..32)
        .map(|_| {
            Arc::new(MeasuredCell {
                measurements: AtomicUsize::new(0),
            })
        })
        .collect();
    let mut overlay = TranscriptOverlay::new(
        cells
            .iter()
            .map(|cell| cell.clone() as Arc<dyn HistoryCell>)
            .collect(),
        RuntimeKeymap::defaults().pager,
    );
    let mut area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 40, /*height*/ 12,
    );
    overlay.render(area, &mut Buffer::empty(area));

    for selection in [Some(30), Some(28), Some(28), None] {
        overlay.set_highlight_cell(selection);
        overlay.render(area, &mut Buffer::empty(area));
    }

    let measurements = || {
        cells
            .iter()
            .map(|cell| cell.measurements.load(Ordering::Relaxed))
            .collect::<Vec<_>>()
    };
    let mut expected = vec![1; cells.len()];
    expected[28] = 3;
    expected[30] = 3;
    assert_eq!(measurements(), expected);

    // Width changes still invalidate every cached height.
    area.width = 24;
    overlay.render(area, &mut Buffer::empty(area));
    for count in &mut expected {
        *count += 1;
    }
    assert_eq!(measurements(), expected);
}
