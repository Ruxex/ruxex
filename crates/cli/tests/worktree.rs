//! Exercises interactive destination policy, startup and fork, and ownership before the first turn.

use anyhow::Context as _;
use pretty_assertions::assert_eq;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;

fn git(cwd: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args([
            "-c",
            "user.name=Worktree Test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

// Retain only the unfinished terminal query, never the full output stream.
#[derive(Default)]
struct TerminalQueries(Vec<u8>);

impl TerminalQueries {
    fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut replies = Vec::new();
        for &byte in bytes {
            self.0.push(byte);
            if self.0 == b"\x1b[6n" {
                replies.extend_from_slice(b"\x1b[1;1R");
                self.0.clear();
            } else if self.0 == b"\x1b[c" {
                replies.extend_from_slice(b"\x1b[?1;2c");
                self.0.clear();
            } else {
                while !b"\x1b[6n".starts_with(&self.0) && !b"\x1b[c".starts_with(&self.0) {
                    self.0.remove(0);
                }
            }
        }
        replies
    }
}

#[test]
fn terminal_queries_reply_once_across_every_split() {
    let input = b"noise\x1b[6n\x1b[c\x1b[6n";
    let expected = b"\x1b[1;1R\x1b[?1;2c\x1b[1;1R";
    for split in 0..=input.len() {
        let mut queries = TerminalQueries::default();
        let mut replies = queries.feed(&input[..split]);
        replies.extend(queries.feed(&input[split..]));
        assert_eq!(replies, expected);
        assert_eq!(queries.feed(b"more output"), Vec::<u8>::new());
    }
}

async fn rejected_start(
    program: &Path,
    args: &[String],
    cwd: &Path,
    env: &HashMap<String, String>,
    expected: &str,
) -> anyhow::Result<String> {
    let spawned = genex_utils_pty::spawn_pty_process(
        &program.to_string_lossy(),
        args,
        cwd,
        env,
        /*arg0*/ &None,
        genex_utils_pty::TerminalSize {
            rows: 40,
            cols: 120,
        },
        &[],
    )
    .await?;
    let mut stdout = spawned.stdout_rx;
    let mut exit = spawned.exit_rx;
    let mut output = String::new();
    let mut queries = TerminalQueries::default();
    let result = tokio::time::timeout(Duration::from_secs(/*secs*/ 45), async {
        loop {
            tokio::select! {
                status = &mut exit, if output.contains(expected) => return Ok::<_, anyhow::Error>(status?),
                bytes = stdout.recv() => {
                    let Some(bytes) = bytes else {
                        anyhow::ensure!(output.contains(expected), "TUI exited before rejection: {output}");
                        return Ok(exit.await?);
                    };
                    output.push_str(&String::from_utf8_lossy(&bytes));
                    let replies = queries.feed(&bytes);
                    if !replies.is_empty() { spawned.session.writer_sender().send(replies).await?; }
                }
            }
        }
    }).await;
    if !matches!(&result, Ok(Ok(code)) if *code != 0) {
        let _ = writeln!(
            std::io::stderr(),
            "worktree rejection failed (timeout={}): expected {:?}; output prefix: {:?}",
            result.is_err(),
            expected.chars().take(/*n*/ 200).collect::<String>(),
            output.chars().take(/*n*/ 4000).collect::<String>()
        );
    }
    if result.is_err() {
        spawned.session.terminate();
    }
    assert_ne!(
        result.with_context(|| format!("rejection timed out: {output}"))??,
        0,
        "{output}"
    );
    Ok(output)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interactive_worktree_start_and_fork_bind_owner_before_turn() -> anyhow::Result<()> {
    let root = TempDir::new()?;
    let root = genex_utils_absolute_path::AbsolutePathBuf::from_absolute_path(root.path())?
        .canonicalize()?
        .into_path_buf();
    let home = root.join("home");
    let source = root.join("source");
    let launcher = root.join("launcher");
    for path in [&home, &source, &launcher, &launcher.join("extra")] {
        fs::create_dir(path)?;
    }
    fs::write(
        source.join("AGENTS.md"),
        "committed destination instructions",
    )?;
    let server = MockServer::start().await;
    fs::create_dir(source.join(".genex"))?;
    fs::write(
        source.join(".genex/config.toml"),
        "model = \"destination-model\"\nanalytics.enabled = false\ncli_auth_credentials_store = \"file\"\n",
    )?;
    git(&source, &["init", "--quiet"])?;
    git(&source, &["add", "."])?;
    git(
        &source,
        &["commit", "--quiet", "--no-gpg-sign", "-m", "initial"],
    )?;
    fs::write(
        home.join("config.toml"),
        format!(
            r#"
cli_auth_credentials_store = "file"
managed_auth_base_url = "{}/source/backend-api"
features.worktrees = true
model_provider = "local"
model = "test-model"
sandbox_mode = "workspace-write"
windows.sandbox = "unelevated"
tui.disable_paste_burst = true
[otel]
metrics_exporter = {{ otlp-http = {{ endpoint = "{}/metrics", protocol = "json" }} }}
[model_providers.local]
name = "local test"
base_url = "{}/v1"
wire_api = "responses"
[projects.{}]
trust_level = "trusted"
[projects.{}]
trust_level = "trusted"
"#,
            server.uri(),
            server.uri(),
            server.uri(),
            serde_json::to_string(&source)?,
            serde_json::to_string(&launcher)?
        ),
    )?;
    fs::write(
        source.join("AGENTS.md"),
        "uncommitted launcher instructions",
    )?;
    fs::write(
        source.join(".genex/config.toml"),
        "model = \"source-model\"\nanalytics.enabled = true\n",
    )?;
    app_test_support::write_api_key_auth(
        &home,
        app_test_support::ApiKeyAuthFixture::new("test-token")
            .account_id("workspace-123")
            .workspace_id("workspace-123")
            .user_id("user-123")
            .plan_type("enterprise"),
        genex_config::types::AuthCredentialsStoreMode::File,
    )?;
    let program = genex_utils_cargo_bin::cargo_bin("genex")?;
    let mut env: HashMap<String, String> = std::env::vars().collect();
    env.insert("GENEX_HOME".into(), home.display().to_string());
    env.insert("GENEX_SQLITE_HOME".into(), home.display().to_string());
    env.insert("NO_PROXY".into(), "127.0.0.1,localhost".into());
    env.insert("no_proxy".into(), "127.0.0.1,localhost".into());
    env.insert("TERM".into(), "xterm-256color".into());
    env.insert("TERM_PROGRAM".into(), "unrecognized-terminal".into());
    env.insert("OTEL_METRIC_EXPORT_INTERVAL".into(), "100".into());
    for key in [
        "GENEX_EXEC_SERVER_URL",
        "GENEX_ACCESS_TOKEN",
        "PROVIDER_API_KEY",
        "GENEX_API_KEY",
        "TMUX",
        "TMUX_PANE",
        "ZELLIJ",
        "ZELLIJ_SESSION_NAME",
        "ZELLIJ_VERSION",
    ] {
        env.remove(key);
    }
    let mut owner = None;
    let mut previous: Vec<String> = Vec::new();
    for (fork, explicit_cd, analytics) in [
        (false, false, false),
        (true, false, false),
        (true, true, false),
        (false, false, true),
    ] {
        let (metric_tx, mut metric_rx) = tokio::sync::mpsc::unbounded_channel();
        Mock::given(wiremock::matchers::path("/metrics"))
            .respond_with(move |_: &wiremock::Request| {
                let _ = metric_tx.send(());
                ResponseTemplate::new(200).set_body_json(serde_json::json!({}))
            })
            .mount(&server)
            .await;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let response = app_test_support::create_final_assistant_message_sse_response("done")?;
        let observed_source = source.clone();
        let observed_pool = home.join("worktrees");
        let observed_previous = previous.clone();
        Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/v1/responses"))
            .respond_with(move |request: &wiremock::Request| {
                let observed = (|| {
                    let checkouts = git(&observed_source, &["worktree", "list", "--porcelain"])?;
                    let checkout = checkouts
                        .lines()
                        .filter_map(|line| line.strip_prefix("worktree "))
                        .find(|path| {
                            Path::new(path).starts_with(&observed_pool)
                                && !observed_previous.contains(&path.to_string())
                        })
                        .context("new managed checkout")?
                        .to_owned();
                    let metadata = git(
                        Path::new(&checkout),
                        &["rev-parse", "--git-path", "genex-thread.json"],
                    )?;
                    let metadata: Value =
                        serde_json::from_slice(&fs::read(Path::new(&checkout).join(metadata))?)?;
                    Ok::<_, anyhow::Error>((request.body.clone(), checkout, metadata))
                })();
                let _ = tx.send(observed);
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(response.clone())
            })
            .mount(&server)
            .await;
        let mut args = vec!["--worktree".to_owned(), "--no-alt-screen".to_owned()];
        if fork {
            args.extend([
                "--add-dir".into(),
                "extra".into(),
                "fork".into(),
                owner.clone().context("previous fork owner")?,
            ]);
        } else {
            args.extend(["--cd".into(), source.display().to_string()]);
        }
        if explicit_cd {
            args.extend(["--cd".into(), source.join(".genex").display().to_string()]);
        }
        if analytics {
            args.extend(["-c".into(), "analytics.enabled=true".into()]);
        }
        args.push("describe checkout".into());
        if previous.is_empty() {
            let mut untrusted_args = args.clone();
            untrusted_args.extend([
                "-c".into(),
                format!(
                    "projects={{{}={{trust_level=\"untrusted\"}}}}",
                    serde_json::to_string(&source)?
                ),
            ]);
            rejected_start(
                &program,
                &untrusted_args,
                &launcher,
                &env,
                "cannot create a checkout from an explicitly untrusted source",
            )
            .await?;
            assert!(!home.join("worktrees").exists());
        }
        let spawned = genex_utils_pty::spawn_pty_process(
            &program.to_string_lossy(),
            &args,
            &launcher,
            &env,
            /*arg0*/ &None,
            genex_utils_pty::TerminalSize {
                rows: 40,
                cols: 120,
            },
            &[],
        )
        .await?;
        let session = spawned.session;
        let mut stdout = spawned.stdout_rx;
        let mut output = String::new();
        let mut queries = TerminalQueries::default();
        let observed = tokio::time::timeout(Duration::from_secs(/*secs*/ 45), async {
            loop {
                tokio::select! {
                    body = rx.recv() => {
                        let (body, checkout, metadata) = body.context("model request")??;
                        let body: Value = serde_json::from_slice(&body)?;
                        return Ok::<_, anyhow::Error>((body, checkout, metadata));
                    }
                    bytes = stdout.recv() => {
                        let bytes = bytes.context("TUI exited before model request")?;
                        let text = String::from_utf8_lossy(&bytes);
                        output.push_str(&text);
                        let replies = queries.feed(&bytes);
                        if !replies.is_empty() { session.writer_sender().send(replies).await?; }
                    }
                }
            }
        })
        .await;
        // The owner binding is asserted from the persisted model request below. Thread-name
        // persistence is maintained by the app-server store, not the legacy session index.
        let renamed = Ok::<(), anyhow::Error>(());
        if analytics {
            tokio::time::timeout(Duration::from_secs(/*secs*/ 10), metric_rx.recv())
                .await?
                .context("metrics export while the TUI is running")?;
        }
        let startup_result = observed.as_ref().map(|result| result.as_ref().map(|_| ()));
        if !matches!(startup_result, Ok(Ok(()))) || renamed.is_err() {
            // Bypass libtest capture and report before tearing down the Windows PTY.
            let _ = writeln!(
                std::io::stderr(),
                "worktree failure (fork={fork}, explicit_cd={explicit_cd}): startup={startup_result:?}, rename={renamed:?}\nTUI output: {:?}",
                output.chars().take(/*n*/ 4000).collect::<String>()
            );
        }
        if matches!(startup_result, Ok(Ok(()))) && renamed.is_ok() {
            session.writer_sender().send(b"/quit\r".to_vec()).await?;
        } else {
            session.terminate();
        }
        let exit =
            tokio::time::timeout(Duration::from_secs(/*secs*/ 10), spawned.exit_rx).await??;
        // Include shutdown output: ConPTY can retain the channel after process exit.
        let _ = tokio::time::timeout(Duration::from_secs(/*secs*/ 1), async {
            while let Some(bytes) = stdout.recv().await {
                output.push_str(&String::from_utf8_lossy(&bytes));
            }
        })
        .await;
        let elevated_handoff = cfg!(windows)
            && output.contains("start the Windows daemon from a non-elevated terminal");
        assert_eq!(exit, i32::from(elevated_handoff), "{output}");
        assert!(!output.contains("The checkout was kept"), "{output}");
        let metrics = server
            .received_requests()
            .await
            .context("requests")?
            .into_iter()
            .filter(|request| request.url.path() == "/metrics")
            .map(|request| serde_json::from_slice::<Value>(&request.body))
            .collect::<serde_json::Result<Vec<_>>>()?;
        if analytics {
            let exported = metrics
                .iter()
                .flat_map(|payload| payload["resourceMetrics"].as_array().into_iter().flatten())
                .flat_map(|resource| resource["scopeMetrics"].as_array().into_iter().flatten())
                .flat_map(|scope| scope["metrics"].as_array().into_iter().flatten())
                .collect::<Vec<_>>();
            let point = exported
                .iter()
                .filter(|metric| metric["name"] == "genex.tui.start")
                .flat_map(|metric| metric["sum"]["dataPoints"].as_array().into_iter().flatten())
                .next()
                .context("genex.tui.start data point")?;
            let tags = point["attributes"]
                .as_array()
                .context("genex.tui.start attributes")?
                .iter()
                .map(|attribute| {
                    Ok((
                        attribute["key"].as_str().context("metric tag key")?,
                        attribute["value"]["stringValue"]
                            .as_str()
                            .context("metric tag value")?,
                    ))
                })
                .collect::<anyhow::Result<HashMap<_, _>>>()?;
            assert_eq!(
                tags,
                HashMap::from([
                    ("app_server_mode", "in_process"),
                    ("terminal_name", "unknown"),
                    ("multiplexer", "none"),
                    ("daemon_selection_reason", "incompatible_option"),
                    ("daemon_auto_start", "disabled"),
                    ("shutdown_grace_setting", "default"),
                ])
            );
        }
        let (body, checkout, metadata) = observed
            .with_context(|| {
                format!(
                    "startup timed out: {}",
                    output.chars().take(/*n*/ 4000).collect::<String>()
                )
            })?
            .with_context(|| {
                format!(
                    "startup output: {}",
                    output.chars().take(/*n*/ 4000).collect::<String>()
                )
            })?;
        renamed.with_context(|| {
            format!(
                "rename output: {}",
                output.chars().take(/*n*/ 4000).collect::<String>()
            )
        })?;
        let next_owner = metadata["ownerThreadId"]
            .as_str()
            .context("owner bound before request")?
            .to_owned();
        assert_ne!(owner.as_ref(), Some(&next_owner));
        assert_eq!(
            git(Path::new(&checkout), &["show", "HEAD:AGENTS.md"])?,
            "committed destination instructions"
        );
        let context = body["input"]
            .as_array()
            .context("message input")?
            .iter()
            .filter_map(|item| item["content"].as_array())
            .flatten()
            .filter_map(|part| part["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if explicit_cd {
            let expected = format!("{checkout}/.genex").replace('\\', "/");
            assert!(context.replace('\\', "/").contains(&expected), "{context}");
        }
        assert!(
            context.contains("committed destination instructions"),
            "{context}"
        );
        assert!(
            !context.contains("uncommitted launcher instructions"),
            "{context}"
        );
        if fork {
            assert!(
                context.contains(&launcher.join("extra").display().to_string()),
                "{context}"
            );
        }
        assert_eq!(body["model"], "destination-model");
        previous.push(checkout);
        owner = Some(next_owner);
        server.reset().await;
    }
    // Source loads its healthy uncommitted config, but the new checkout loads malformed HEAD.
    fs::write(source.join(".genex/config.toml"), "not = [valid toml")?;
    git(
        &source,
        &[
            "commit",
            "--quiet",
            "--no-gpg-sign",
            "-am",
            "invalid destination",
        ],
    )?;
    fs::write(source.join(".genex/config.toml"), "")?;
    let before = git(&source, &["worktree", "list", "--porcelain"])?;
    let output = rejected_start(
        &program,
        &[
            "--worktree".into(),
            "--no-alt-screen".into(),
            "--cd".into(),
            source.display().to_string(),
        ],
        &launcher,
        &env,
        "Do not use --force.",
    )
    .await?;
    let after = git(&source, &["worktree", "list", "--porcelain"])?;
    let retained = after
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .find(|path| {
            !before
                .lines()
                .any(|line| line.strip_prefix("worktree ") == Some(*path))
        })
        .context("retained failed checkout")?;
    assert!(
        output.contains(&format!(
            "{:?}",
            Path::new(&retained.replace('/', std::path::MAIN_SEPARATOR_STR))
        )),
        "{output}"
    );
    assert!(output.contains("The checkout was kept"), "{output}");
    assert!(
        output.contains("worktree remove <checkout-path>"),
        "{output}"
    );
    Ok(())
}
