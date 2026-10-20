use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use anyhow::ensure;
use pretty_assertions::assert_eq;
use ruxex_login::RuxexAuth;
use ruxex_model_provider::CURRENT_PROVIDER_PROTOCOL_VERSION;
use ruxex_model_provider_info::ModelProviderInfo;
use ruxex_models_manager::test_support::test_models_response;
use tempfile::TempDir;
use uuid::Uuid;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

// Full startup continues after the composer first appears and can be slower under Rosetta in CI.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 30);
const FOCUS_INPUT_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 5);
const FOCUS_PROBE_INPUT: &str = "focus-palette-24527";
const PTY_SCROLLBACK_LINES: usize = 512;

#[test]
fn focus_gained_with_unanswered_palette_queries_preserves_immediate_input() -> Result<()> {
    let repo_root = ruxex_utils_cargo_bin::repo_root()?;
    let ruxex_home = tempfile::tempdir()?;
    write_test_config(ruxex_home.path(), &repo_root)?;

    let mut terminal = PtyRuxex::start(&repo_root, ruxex_home, &[])?;
    terminal.wait_for_startup()?;

    let startup_output_len = terminal.output.len();
    let focus_started = Instant::now();
    terminal.write_input(format!("\u{1b}[I{FOCUS_PROBE_INPUT}").as_bytes())?;
    terminal.wait_for_focus_input(FOCUS_PROBE_INPUT, focus_started, startup_output_len)?;

    let delayed_input = format!("{FOCUS_PROBE_INPUT}-delayed");
    let delayed_focus_started = Instant::now();
    terminal.write_input(b"\x1b[I")?;
    terminal.read_output(Duration::from_millis(/*millis*/ 20))?;
    terminal.write_input(delayed_input.as_bytes())?;
    terminal.wait_for_focus_input(&delayed_input, delayed_focus_started, startup_output_len)?;

    Ok(())
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interactive_startup_honors_ruxex_home_symlink_opt_out() -> Result<()> {
    use core_test_support::responses;
    use wiremock::matchers::body_string_contains;

    core_test_support::skip_if_sandbox!(Ok(()));
    let workspace = tempfile::tempdir()?;
    let workspace_path = workspace.path().canonicalize()?;
    let ruxex_home = tempfile::tempdir()?;
    let target = tempfile::tempdir()?;
    let visualizations = ruxex_home.path().join("visualizations");
    std::os::unix::fs::symlink(target.path(), &visualizations)?;
    write_test_config(ruxex_home.path(), &workspace_path)?;

    let server = responses::start_mock_server().await;
    mount_test_models(&server).await?;
    // Automatic thread-title requests must not consume the tool responses.
    let _title_mock = responses::mount_sse_once_match(
        &server,
        body_string_contains(r#"\"thread_source\":\"system\""#),
        responses::sse(vec![
            responses::ev_assistant_message("title", r#"{"title":"Symlink probe"}"#),
            responses::ev_completed("title"),
        ]),
    )
    .await;
    let _write_mock = responses::mount_sse_once_match(
        &server,
        body_string_contains(r#"\"thread_source\":\"user\""#),
        responses::sse(vec![
            responses::ev_exec_command_call_with_args(
                "write",
                &serde_json::json!({
                    "cmd": "printf ready > ready.txt",
                    "workdir": visualizations,
                    "shell": "/bin/sh",
                    "login": false,
                }),
            ),
            responses::ev_completed("startup"),
        ]),
    )
    .await;
    let completion_mock = responses::mount_sse_once_match(
        &server,
        body_string_contains(r#"\"thread_source\":\"user\""#),
        responses::sse(vec![
            responses::ev_assistant_message("ready", "symlink-opt-out-ready"),
            responses::ev_completed("complete"),
        ]),
    )
    .await;
    let base_url = server.uri();
    let visualizations = toml::Value::String(visualizations.display().to_string());
    let config_path = ruxex_home.path().join("config.toml");
    let config = std::fs::read_to_string(&config_path)?
        .replace("name = \"Ruxex TUI Test Provider\"", "name = \"Mock\"")
        .replace(
            "base_url = \"http://127.0.0.1:1/v1\"",
            &format!("base_url = \"{base_url}/v1\""),
        );
    std::fs::write(
        config_path,
        format!(
            "allow_symlinked_ruxex_home = true\n\
             sandbox_mode = \"workspace-write\"\n\
             approval_policy = \"never\"\n\
             {config}\n\
             [sandbox_workspace_write]\nwritable_roots = [{visualizations}]\n"
        ),
    )?;
    // A fresh home has no daemon. Writing through the link exercises the embedded server's
    // local executor and its sandbox policy.
    // This test exercises the embedded app-server and sandbox path. Avoid starting the managed
    // daemon so the provider mock remains in the test process on every supported platform.
    let mut terminal = PtyRuxex::start(
        &workspace_path,
        ruxex_home,
        &["--no-daemon", "Write ready.txt"],
    )?;
    terminal.wait_for_startup()?;
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        terminal.read_output(Duration::from_millis(/*millis*/ 50))?;
        if terminal.screen_contains("symlink-opt-out-ready") {
            let tool_output = completion_mock
                .function_call_output_text("write")
                .context("missing symlink write tool output")?;
            ensure!(
                tool_output.contains("Process exited with code 0"),
                "{tool_output}"
            );
            assert_eq!(std::fs::read(target.path().join("ready.txt"))?, b"ready");
            return Ok(());
        }
        if terminal.child.try_wait()?.is_some() {
            break;
        }
    }
    bail!(
        "interactive startup did not honor the symlink opt-out; screen:\n{}",
        terminal.screen_contents(),
    );
}

pub(super) struct PtyRuxex {
    master: File,
    child: Child,
    stderr_output: Arc<Mutex<Vec<u8>>>,
    stderr_reader: Option<JoinHandle<()>>,
    parser: vt100::Parser,
    output: Vec<u8>,
    command_debug: String,
    repo_root: PathBuf,
    config_snapshot: String,
    cursor_answered: bool,
    palette_answered: bool,
    keyboard_answered: bool,
    _ruxex_home: TempDir,
}

impl PtyRuxex {
    pub(super) fn start(
        repo_root: &Path,
        ruxex_home: TempDir,
        extra_args: &[&str],
    ) -> Result<Self> {
        let mut master_fd = -1;
        let mut slave_fd = -1;
        let mut window_size = libc::winsize {
            ws_row: 32,
            ws_col: 120,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        // SAFETY: `openpty` initializes both file descriptors on success, and the supplied window
        // size remains valid for the duration of the call.
        let result = unsafe {
            libc::openpty(
                &mut master_fd,
                &mut slave_fd,
                /*name*/ std::ptr::null_mut(),
                /*termp*/ std::ptr::null_mut(),
                &raw mut window_size,
            )
        };
        if result == -1 {
            return Err(std::io::Error::last_os_error()).context("open focus-test pseudo-terminal");
        }

        // SAFETY: a successful `openpty` transfers ownership of both unique file descriptors.
        let master = File::from(unsafe { OwnedFd::from_raw_fd(master_fd) });
        // SAFETY: `slave_fd` is the second unique descriptor initialized by `openpty`.
        let slave = File::from(unsafe { OwnedFd::from_raw_fd(slave_fd) });
        let stdin = slave.try_clone().context("clone pseudo-terminal stdin")?;
        let stdout = slave.try_clone().context("clone pseudo-terminal stdout")?;

        let ruxex = ruxex_utils_cargo_bin::cargo_bin("ruxex-provider-host-example")?;
        let config_snapshot = std::fs::read_to_string(ruxex_home.path().join("config.toml"))?;
        let config: toml::Value = toml::from_str(&config_snapshot)?;
        let provider_info_value = serde_json::to_value(
            config
                .get("model_provider")
                .context("focus-test config is missing model_provider")?,
        )?;
        let provider_info: ModelProviderInfo = serde_json::from_value(provider_info_value)?;
        let provider_info_json = serde_json::to_string(&provider_info)?;
        let auth = RuxexAuth::from_api_key("focus-palette-test");
        let mut models_cache = ruxex_model_provider::test_support::models_cache_entry(
            &provider_info,
            Some(&auth),
            test_models_response()?.models,
        );
        models_cache.identity = ruxex_model_provider::provider_models_identity::identity(
            provider_info.id.as_deref(),
            &provider_info,
            Some(&auth),
        )
        .ok();
        std::fs::write(
            ruxex_home.path().join("models_cache.json"),
            serde_json::to_vec(&models_cache)?,
        )?;
        let command_debug = format!(
            "{} --model provider-model-d {:?} --no-alt-screen -C {}",
            ruxex.display(),
            extra_args,
            repo_root.display(),
        );
        let mut child = Command::new(&ruxex)
            .arg("--model")
            .arg("provider-model-d")
            .args(extra_args)
            .arg("--no-alt-screen")
            .arg("-C")
            .arg(repo_root)
            .env("TERM", "xterm-256color")
            .env("PROVIDER_API_KEY", "focus-palette-test")
            .env("RUXEX_HOME", ruxex_home.path())
            .env("RUXEX_TEST_PROVIDER_INFO", provider_info_json)
            .stdin(stdin)
            .stdout(stdout)
            .stderr(Stdio::piped())
            .spawn()
            .context("start Ruxex in focus-test pseudo-terminal")?;
        let mut child_stderr = child
            .stderr
            .take()
            .context("capture Ruxex stderr in focus-test pseudo-terminal")?;
        let stderr_output = Arc::new(Mutex::new(Vec::new()));
        let stderr_output_for_reader = Arc::clone(&stderr_output);
        let stderr_reader = std::thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            loop {
                match child_stderr.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if let Ok(mut output) = stderr_output_for_reader.lock() {
                            output.extend_from_slice(&buffer[..count]);
                        } else {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            master,
            child,
            stderr_output,
            stderr_reader: Some(stderr_reader),
            parser: vt100::Parser::new(
                /*rows*/ 32,
                /*cols*/ 120,
                /*scrollback_len*/ PTY_SCROLLBACK_LINES,
            ),
            output: Vec::new(),
            command_debug,
            repo_root: repo_root.to_path_buf(),
            config_snapshot,
            cursor_answered: false,
            palette_answered: false,
            keyboard_answered: false,
            _ruxex_home: ruxex_home,
        })
    }

    pub(super) fn wait_for_startup(&mut self) -> Result<()> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        while Instant::now() < deadline {
            self.read_output(Duration::from_millis(/*millis*/ 50))?;
            self.answer_startup_queries()?;

            if self.palette_answered && self.screen_contains("provider Provider") {
                return Ok(());
            }

            if let Some(status) = self.child.try_wait()? {
                bail!(
                    "{}",
                    self.failure_report("Ruxex exited before the focus test started", Some(status))
                );
            }
        }

        let status = self.child.try_wait()?;
        bail!(
            "{}",
            self.failure_report(
                &format!("Ruxex did not initialize within {:?}", STARTUP_TIMEOUT),
                status,
            )
        );
    }

    fn wait_for_focus_input(
        &mut self,
        input: &str,
        focus_started: Instant,
        startup_output_len: usize,
    ) -> Result<()> {
        while focus_started.elapsed() < FOCUS_INPUT_TIMEOUT {
            self.read_output(Duration::from_millis(/*millis*/ 20))?;
            let focus_output = &self.output[startup_output_len..];
            ensure!(
                !contains_bytes(focus_output, b"\x1b]10;?")
                    && !contains_bytes(focus_output, b"\x1b]11;?"),
                "focus regain queried terminal colors after the startup palette was cached",
            );
            if self.screen_contains(input) {
                return Ok(());
            }
        }

        bail!(
            "focus-time palette refresh blocked or discarded {input:?} for more than {:?}; \
             screen:\n{}",
            FOCUS_INPUT_TIMEOUT,
            self.screen_contents(),
        );
    }

    fn answer_startup_queries(&mut self) -> Result<()> {
        if !self.cursor_answered && contains_bytes(&self.output, b"\x1b[6n") {
            self.write_input(b"\x1b[1;1R")?;
            self.cursor_answered = true;
        }

        if !self.keyboard_answered && contains_bytes(&self.output, b"\x1b[?u") {
            self.write_input(b"\x1b[?0u\x1b[?1;2c")?;
            self.keyboard_answered = true;
        }

        if !self.palette_answered
            && contains_bytes(&self.output, b"\x1b]10;?")
            && contains_bytes(&self.output, b"\x1b]11;?")
        {
            self.write_input(b"\x1b]10;rgb:ffff/ffff/ffff\x1b\\\x1b]11;rgb:0000/0000/0000\x1b\\")?;
            self.palette_answered = true;
        }

        Ok(())
    }

    pub(super) fn read_output(&mut self, timeout: Duration) -> Result<()> {
        let timeout_ms = timeout.as_millis().min(libc::c_int::MAX as u128) as libc::c_int;
        let mut descriptor = libc::pollfd {
            fd: self.master.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };

        // SAFETY: `descriptor` points to one initialized poll descriptor.
        let ready = unsafe {
            libc::poll(&mut descriptor, /*nfds*/ 1, timeout_ms)
        };
        if ready == -1 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                return Ok(());
            }
            return Err(error).context("poll focus-test pseudo-terminal");
        }
        if ready == 0 || descriptor.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) == 0 {
            return Ok(());
        }

        let mut chunk = [0_u8; 8192];
        let count = match self.master.read(&mut chunk) {
            Ok(count) => count,
            // A PTY master reports EIO after its slave closes. The process may have written its
            // final diagnostic just before that close, so treat EIO as EOF after draining the
            // bytes returned with POLLIN/POLLHUP rather than hiding the useful error.
            Err(error) if error.raw_os_error() == Some(libc::EIO) => return Ok(()),
            Err(error) => return Err(error).context("read focus-test pseudo-terminal"),
        };
        self.output.extend_from_slice(&chunk[..count]);
        self.parser.process(&chunk[..count]);
        Ok(())
    }

    pub(super) fn write_input(&mut self, bytes: &[u8]) -> Result<()> {
        self.master.write_all(bytes)?;
        self.master.flush()?;
        Ok(())
    }

    pub(super) fn screen_contains(&self, text: &str) -> bool {
        self.parser.screen().contents().contains(text)
    }

    pub(super) fn screen_contents(&self) -> String {
        self.parser.screen().contents()
    }

    pub(super) fn wait_for_screen(&mut self, text: &str) -> Result<()> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        while Instant::now() < deadline {
            self.read_output(Duration::from_millis(/*millis*/ 50))?;
            self.answer_startup_queries()?;
            if self.screen_contains(text) {
                return Ok(());
            }
            if let Some(status) = self.child.try_wait()? {
                bail!(
                    "{}",
                    self.failure_report(
                        &format!("Ruxex exited while waiting for {text:?}"),
                        Some(status),
                    )
                );
            }
        }
        let status = self.child.try_wait()?;
        bail!(
            "{}",
            self.failure_report(&format!("missing {text:?}"), status)
        )
    }

    pub(super) fn ensure_running(&mut self) -> Result<()> {
        if let Some(status) = self.child.try_wait()? {
            bail!(
                "{}",
                self.failure_report("Ruxex exited unexpectedly", Some(status))
            );
        }
        Ok(())
    }

    fn failure_report(&mut self, reason: &str, status: Option<ExitStatus>) -> String {
        if status.is_some() {
            self.finish_stderr_capture();
        }
        let status = status
            .map(|status| format!("{status:?}"))
            .unwrap_or_else(|| "still running or unavailable".to_string());
        let stderr = self
            .stderr_output
            .lock()
            .map(|output| diagnostic_text(&output))
            .unwrap_or_else(|_| "<stderr capture lock poisoned>".to_string());
        let log_path = self._ruxex_home.path().join("log/ruxex-tui.log");
        let log = read_diagnostic_file(&log_path);
        let home_inventory = diagnostic_home_inventory(self._ruxex_home.path());
        format!(
            "{reason}\n\
             child status: {status}\n\
             command: {}\n\
             repo root: {}\n\
             RUXEX_HOME: {}\n\
             environment: TERM=xterm-256color, PROVIDER_API_KEY=<set>, \
RUXEX_TEST_PROVIDER_INFO=<set>\n\
             config.toml:\n{}\n\
             child stderr:\n{}\n\
             TUI log ({}):\n{}\n\
             RUXEX_HOME inventory:\n{}\n\
             PTY screen:\n{}\n\
             PTY raw output tail:\n{}",
            self.command_debug,
            self.repo_root.display(),
            self._ruxex_home.path().display(),
            self.config_snapshot,
            stderr,
            log_path.display(),
            log,
            home_inventory,
            self.screen_contents(),
            diagnostic_text(&self.output),
        )
    }

    fn finish_stderr_capture(&mut self) {
        if let Some(reader) = self.stderr_reader.take() {
            let _ = reader.join();
        }
    }
}

impl Drop for PtyRuxex {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.finish_stderr_capture();
    }
}

fn diagnostic_text(bytes: &[u8]) -> String {
    const MAX_BYTES: usize = 16 * 1024;
    let start = bytes.len().saturating_sub(MAX_BYTES);
    String::from_utf8_lossy(&bytes[start..])
        .escape_default()
        .to_string()
}

fn read_diagnostic_file(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => diagnostic_text(&bytes),
        Err(error) => format!("<unavailable: {error}>"),
    }
}

fn diagnostic_home_inventory(root: &Path) -> String {
    let mut entries = Vec::new();
    collect_diagnostic_entries(root, root, /*depth*/ 0, &mut entries);
    if entries.is_empty() {
        return "<empty or unavailable>".to_string();
    }
    entries.join("\n")
}

fn collect_diagnostic_entries(root: &Path, path: &Path, depth: usize, entries: &mut Vec<String>) {
    if depth > 2 {
        return;
    }
    let Ok(directory) = std::fs::read_dir(path) else {
        return;
    };
    let mut children: Vec<_> = directory.flatten().collect();
    children.sort_by_key(|entry| entry.file_name());
    for entry in children {
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let metadata = std::fs::symlink_metadata(&path).ok();
        let size = metadata
            .as_ref()
            .filter(|metadata| metadata.is_file())
            .map(|metadata| format!(" ({} bytes)", metadata.len()))
            .unwrap_or_default();
        entries.push(format!("{}{}", relative.display(), size));
        if metadata.is_some_and(|metadata| metadata.is_dir()) {
            collect_diagnostic_entries(root, &path, depth + 1, entries);
        }
    }
}

fn contains_bytes(buffer: &[u8], needle: &[u8]) -> bool {
    buffer.windows(needle.len()).any(|window| window == needle)
}

pub(super) fn write_test_config(ruxex_home: &Path, repo_root: &Path) -> Result<()> {
    let repo_root = repo_root.display();
    let config = format!(
        "model = \"provider-model-d\"\ncli_auth_credentials_store = \"file\"\n\
         suppress_unstable_features_warning = true\nanalytics.enabled = false\n\n\
         [model_provider]\nid = \"ruxex-tui-test-provider\"\nname = \"Ruxex TUI Test Provider\"\nbase_url = \"http://127.0.0.1:1/v1\"\nwire_api = \"responses\"\nsupports_websockets = false\n\n\
         [projects.\"{repo_root}\"]\ntrust_level = \"trusted\"\n"
    );
    std::fs::write(ruxex_home.join("config.toml"), config)
        .context("write focus-test Ruxex configuration")?;
    std::fs::write(
        ruxex_home.join("auth.json"),
        r#"{"auth_mode":"apiKey","api_key":"focus-palette-test"}"#,
    )
    .context("write focus-test API-key authentication")?;
    std::fs::write(
        ruxex_home.join("ruxex-home.json"),
        serde_json::json!({
            "formatVersion": 2,
            "providerKind": "ruxex-tui-test-provider",
            "providerProtocolVersion": CURRENT_PROVIDER_PROTOCOL_VERSION,
            "serverSignature": Uuid::new_v4().to_string(),
        })
        .to_string(),
    )
    .context("write focus-test provider-home marker")?;
    Ok(())
}

pub(super) async fn mount_test_models(server: &MockServer) -> Result<()> {
    // The app server refreshes the catalog concurrently with the TUI bootstrap. Always return a
    // non-empty catalog so either request can safely win that race on slower CI runners.
    // `responses::start_mock_server` installs an empty catch-all `/models` mock first. Reset it
    // before installing this fixture, otherwise wiremock can legitimately return the empty
    // catalog and persist it over the seeded cache on platforms where the cache is not reused.
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(ruxex_models_manager::test_support::test_models_response()?),
        )
        .mount(server)
        .await;
    Ok(())
}

#[test]
fn no_daemon_skips_startup_and_discovery() -> Result<()> {
    for running in [false, true] {
        let workspace = tempfile::tempdir()?;
        let home = tempfile::tempdir()?;
        write_test_config(home.path(), workspace.path())?;
        let config = home.path().join("config.toml");
        let contents = std::fs::read_to_string(&config)?;
        std::fs::write(
            config,
            format!("features.daemon_auto_start = true\n{contents}"),
        )?;
        let socket_path = ruxex_app_server_client::app_server_control_socket_path(home.path())?;
        std::fs::create_dir_all(socket_path.as_path().parent().unwrap())?;
        let listener = if running {
            let listener = std::os::unix::net::UnixListener::bind(socket_path.as_path())?;
            listener.set_nonblocking(true)?;
            Some(listener)
        } else {
            None
        };
        let mut terminal = PtyRuxex::start(workspace.path(), home, &["--no-daemon"])?;
        terminal.wait_for_startup()?;
        terminal.write_input(b"/status")?;
        terminal.wait_for_screen("show current session configuration")?;
        terminal.read_output(Duration::from_millis(/*millis*/ 200))?;
        terminal.write_input(b"\r")?;
        terminal.wait_for_screen("Model:")?;
        ensure!(
            !terminal
                ._ruxex_home
                .path()
                .join("app-server-daemon")
                .exists()
        );
        if let Some(listener) = listener {
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
        ensure!(
            !String::from_utf8_lossy(&terminal.output)
                .contains("Running without the shared background server")
        );
    }
    Ok(())
}
