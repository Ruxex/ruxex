use std::process::Command;

#[test]
fn preview_binary_renders_fixture_without_starting_tui() {
    let binary = ruxex_utils_cargo_bin::cargo_bin("ruxex-tui-preview")
        .expect("preview binary should be available");
    let output_dir = tempfile::tempdir().expect("temporary output directory");
    let output_path = output_dir.path().join("preview.svg");
    let fixture = ruxex_utils_cargo_bin::find_resource!("tests/fixtures/preview.toml")
        .expect("fixture should be available");

    let result = Command::new(binary)
        .args([
            "--design",
            fixture.to_str().expect("fixture path is UTF-8"),
            "--scene",
            "all",
            "--output",
            output_path.to_str().expect("output path is UTF-8"),
        ])
        .output()
        .expect("run preview binary");

    assert!(result.status.success(), "stderr: {:?}", result.stderr);
    let svg = std::fs::read_to_string(output_path).expect("read SVG");
    assert!(svg.contains("RUXEX PREVIEW"));
    assert!(svg.contains("CONVERSATION"));
    assert!(svg.contains("#58a6ff"));
}
