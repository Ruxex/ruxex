use std::path::Path;

use anyhow::Result;
use tempfile::TempDir;

fn genex_command(genex_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(genex_utils_cargo_bin::cargo_bin("genex")?);
    cmd.env("GENEX_HOME", genex_home);
    Ok(cmd)
}

#[test]
fn debug_models_default_prints_json_without_auth() -> Result<()> {
    let genex_home = TempDir::new()?;
    let mut cmd = genex_command(genex_home.path())?;
    let output = cmd.args(["debug", "models"]).output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let value: serde_json::Value = serde_json::from_str(&stdout)?;
    assert!(value["models"].is_array());

    Ok(())
}
