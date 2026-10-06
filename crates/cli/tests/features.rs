use std::path::Path;

use anyhow::Result;
use predicates::str::contains;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

fn genex_command(genex_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(genex_utils_cargo_bin::cargo_bin("genex")?);
    cmd.env("GENEX_HOME", genex_home);
    Ok(cmd)
}

#[test]
fn strict_config_rejects_unknown_config_override() -> Result<()> {
    let genex_home = TempDir::new()?;

    let mut cmd = genex_command(genex_home.path())?;
    cmd.args(["--strict-config", "-c", "foo=bar", "exec", "hello"])
        .assert()
        .failure()
        .stderr(contains("unknown configuration field"));

    Ok(())
}

#[test]
fn interactive_validates_config_before_requiring_terminal() -> Result<()> {
    let cases: &[(&[&str], &str, &str, &str)] = &[
        (
            &[],
            "config.toml",
            "model = [",
            "Error loading configuration",
        ),
        (
            &["--strict-config"],
            "config.toml",
            "unknown_key = true",
            "unknown configuration field",
        ),
        (
            &["--strict-config", "-c", "foo=bar"],
            "config.toml",
            "",
            "unknown configuration field",
        ),
        (
            &["--profile", "work"],
            "work.config.toml",
            "model = [",
            "work.config.toml",
        ),
        (
            &["-c", "model_provider=\"missing\""],
            "config.toml",
            "",
            "Model provider `missing` not found",
        ),
        (&[], "config.toml", "", "stdin is not a terminal"),
    ];

    for &(args, config_file, contents, expected_error) in cases {
        let genex_home = TempDir::new()?;
        std::fs::write(genex_home.path().join(config_file), contents)?;

        let mut cmd = genex_command(genex_home.path())?;
        cmd.env("TERM", "xterm-256color")
            .current_dir(genex_home.path())
            .args(args)
            .assert()
            .failure()
            .stderr(contains(expected_error));
    }

    Ok(())
}

#[tokio::test]
async fn features_enable_writes_feature_flag_to_config() -> Result<()> {
    let genex_home = TempDir::new()?;

    for feature in ["unified_exec"] {
        let mut cmd = genex_command(genex_home.path())?;
        cmd.args(["features", "enable", feature])
            .assert()
            .success()
            .stdout(contains(format!(
                "Enabled feature `{feature}` in config.toml."
            )));

        let config = std::fs::read_to_string(genex_home.path().join("config.toml"))?;
        assert!(config.contains("[features]"));
        assert!(config.contains(&format!("{feature} = true")));
    }

    Ok(())
}

#[tokio::test]
async fn features_disable_writes_feature_flag_to_config() -> Result<()> {
    let genex_home = TempDir::new()?;

    let mut cmd = genex_command(genex_home.path())?;
    cmd.args(["features", "disable", "shell_tool"])
        .assert()
        .success()
        .stdout(contains("Disabled feature `shell_tool` in config.toml."));

    let config = std::fs::read_to_string(genex_home.path().join("config.toml"))?;
    assert!(config.contains("[features]"));
    assert!(config.contains("shell_tool = false"));

    Ok(())
}

#[tokio::test]
async fn features_enable_under_development_feature_prints_warning() -> Result<()> {
    let genex_home = TempDir::new()?;

    let mut cmd = genex_command(genex_home.path())?;
    cmd.args(["features", "enable", "runtime_metrics"])
        .assert()
        .success()
        .stderr(contains(
            "Under-development features enabled: runtime_metrics.",
        ));

    Ok(())
}

#[tokio::test]
async fn features_list_is_sorted_alphabetically_by_feature_name() -> Result<()> {
    let genex_home = TempDir::new()?;

    let mut cmd = genex_command(genex_home.path())?;
    let output = cmd
        .args(["features", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output)?;

    let actual_names = stdout
        .lines()
        .map(|line| {
            line.split_once("  ")
                .map(|(name, _)| name.trim_end().to_string())
                .expect("feature list output should contain aligned columns")
        })
        .collect::<Vec<_>>();
    let mut expected_names = actual_names.clone();
    expected_names.sort();

    assert_eq!(actual_names, expected_names);

    Ok(())
}

#[test]
fn no_daemon_rejects_agents_and_explicit_remote_targets() -> Result<()> {
    for args in [
        "--no-daemon agents",
        "agents --no-daemon",
        "--no-daemon queue --thread example --message hello",
        "--no-daemon --remote ws://localhost:9999 agents",
        "--no-daemon --remote ws://localhost:9999",
        "--no-daemon --remote ws://localhost:9999 archive example",
        "--no-daemon --remote ws://localhost:9999 queue --thread example --message hello",
        "--remote ws://localhost:9999 resume --no-daemon --last",
        "--no-daemon fork --remote ws://localhost:9999 session-name",
    ] {
        let args = args.split_whitespace().collect::<Vec<_>>();
        let home = TempDir::new()?;
        let expected = if args.contains(&"agents") {
            "--no-daemon cannot be used with genex agents."
        } else if args.contains(&"queue") && !args.contains(&"--remote") {
            "--no-daemon cannot be used with genex queue."
        } else {
            "--no-daemon cannot be used with --remote."
        };
        genex_command(home.path())?
            .args(args)
            .assert()
            .failure()
            .stderr(contains(expected));
        assert!(!home.path().join("app-server-daemon").exists());
    }
    Ok(())
}
