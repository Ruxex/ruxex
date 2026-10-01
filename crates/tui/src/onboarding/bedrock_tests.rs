use super::*;
use pretty_assertions::assert_eq;

fn render_visible(state: &BedrockState) -> String {
    let area = Rect::new(0, 0, 72, 24);
    let mut buffer = Buffer::empty(area);
    state.render(area, &mut buffer, /*error*/ None);
    let mut rows = (area.top()..area.bottom())
        .map(|row| {
            (area.left()..area.right())
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>();
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    rows.join("\n")
}
