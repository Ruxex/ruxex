//! Snapshot coverage for draft text in the composer.

use super::tests::new_test_composer;
use crate::render::renderable::Renderable;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

#[test]
fn draft_composer_snapshot() {
    crate::terminal_palette::with_test_default_colors(
        crate::terminal_probe::DefaultColors {
            fg: (230, 216, 255),
            bg: (36, 27, 53),
        },
        || {
            {
                let (mut composer, _rx) = new_test_composer();
                composer.set_text_content(
                    "Explore the night sky".to_string(),
                    Vec::new(),
                    Vec::new(),
                );
                let width = 60;
                let area = Rect::new(
                    /*x*/ 0,
                    /*y*/ 0,
                    width,
                    composer.desired_height(width),
                );
                let mut buffer = Buffer::empty(area);
                composer.render(area, &mut buffer);
                let rows = buffer
                    .content
                    .chunks(usize::from(width))
                    .map(|row| {
                        row.iter()
                            .map(ratatui::buffer::Cell::symbol)
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                insta::assert_snapshot!("draft_composer", rows);
            }
        },
    );
}
