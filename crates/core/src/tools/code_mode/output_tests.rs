use std::time::Duration;

use ruxex_protocol::models::FunctionCallOutputContentItem;

use super::CodeModeToolOutput;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolOutput;

#[test]
fn handler_duration_updates_only_the_code_mode_header() {
    let output = FunctionToolOutput::from_content(
        vec![FunctionCallOutputContentItem::InputText {
            text: "ready".to_string(),
        }],
        Some(true),
    );
    let mut output = CodeModeToolOutput::new(
        output,
        "Script completed".to_string(),
        Duration::from_millis(1234),
        Some(Duration::from_millis(1001)),
    );
    output.set_handler_duration_ms(2345);

    assert_eq!(
        output.log_output(),
        "Script completed\nWall time 2.345 seconds (code-mode 1.001 seconds; overhead 1.344 seconds)\nOutput:\nready"
    );
}
