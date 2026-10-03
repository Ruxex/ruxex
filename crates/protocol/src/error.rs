use crate::ThreadId;
pub use crate::auth::RefreshTokenFailedError;
pub use crate::auth::RefreshTokenFailedReason;
use crate::exec_output::ExecToolCallOutput;
use crate::network_policy::NetworkPolicyDecisionPayload;
use crate::protocol::GenexErrorInfo;
use crate::protocol::ErrorEvent;
use crate::protocol::MisalignmentErrorDetails;
use crate::protocol::TruncationPolicy;
use genex_async_utils::CancelErr;
use genex_http_client::HttpError;
use genex_utils_string::truncate_middle_chars;
use genex_utils_string::truncate_middle_with_token_budget;
use http::StatusCode;
use serde_json;
use std::fmt;
use std::io;
use std::time::Duration;
use strum_macros::EnumDiscriminants;
use thiserror::Error;
use tokio::task::JoinError;

pub type Result<T> = std::result::Result<T, GenexErr>;

/// Limit UI error messages to a reasonable size while keeping useful context.
const ERROR_MESSAGE_UI_MAX_BYTES: usize = 2 * 1024;

#[derive(Error, Debug)]
pub enum SandboxErr {
    /// Error from sandbox execution
    #[error(
        "sandbox denied exec error, exit code: {}, stdout: {}, stderr: {}",
        .output.exit_code, .output.stdout.text, .output.stderr.text
    )]
    Denied {
        output: Box<ExecToolCallOutput>,
        network_policy_decision: Option<NetworkPolicyDecisionPayload>,
    },

    /// Error from linux seccomp filter setup
    #[cfg(target_os = "linux")]
    #[error("seccomp setup error")]
    SeccompInstall(#[from] seccompiler::Error),

    /// Error from linux seccomp backend
    #[cfg(target_os = "linux")]
    #[error("seccomp backend error")]
    SeccompBackend(#[from] seccompiler::BackendError),

    /// Command timed out
    #[error("command timed out")]
    Timeout { output: Box<ExecToolCallOutput> },

    /// Command was killed by a signal
    #[error("command was killed by a signal")]
    Signal(i32),

    /// Error from linux landlock
    #[error("Landlock was not able to fully enforce all sandbox rules")]
    LandlockRestrict,
}

pub struct GenexErr {
    details: GenexErrorDetails,
    retry_delay: Option<Duration>,
}

/// The semantic category and diagnostic payload for a [`GenexErr`].
#[derive(Error, Debug, EnumDiscriminants)]
#[strum_discriminants(name(GenexErrKind))]
#[strum_discriminants(derive(serde::Serialize))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
#[strum_discriminants(doc = "The payload-free semantic category used for analytics.")]
pub enum GenexErrorDetails {
    #[error("turn aborted. Something went wrong.")]
    TurnAborted,

    #[error("shared rollout token budget exhausted")]
    SessionBudgetExceeded,

    /// Returned by ResponsesClient when the SSE stream disconnects or errors out **after** the HTTP
    /// handshake has succeeded but **before** it finished emitting `response.completed`.
    ///
    /// The Session loop treats this as a transient error and will automatically retry the turn.
    #[error("stream disconnected before completion: {0}")]
    Stream(String),
    /// A retryable upstream rate limit received inside the response stream.
    #[error("rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    // The iOS input-limit classifier matches this message's ASCII prefix.
    #[error(
        "Genex ran out of room in the model's context window. Start a new thread or clear earlier history before retrying."
    )]
    ContextWindowExceeded,
    #[error("no thread with id: {0}")]
    ThreadNotFound(ThreadId),
    #[error("agent thread limit reached")]
    AgentLimitReached { max_threads: usize },
    #[error("session configured event was not the first event in the stream")]
    SessionConfiguredNotFirstEvent,
    /// Returned by run_command_stream when the spawned child process timed out (10s).
    #[error("timeout waiting for child process to exit")]
    Timeout,
    #[error("request timed out")]
    RequestTimeout,
    /// Returned by run_command_stream when the child could not be spawned (its stdout/stderr pipes
    /// could not be captured). Analogous to the previous `GenexError::Spawn` variant.
    #[error("spawn failed: child stdout/stderr not captured")]
    Spawn,
    /// Returned by run_command_stream when the user pressed Ctrl-C (SIGINT). Session uses this to
    /// surface a polite FunctionCallOutput back to the model instead of crashing the CLI.
    #[error("interrupted (Ctrl-C). Something went wrong.")]
    Interrupted,
    /// Unexpected HTTP status code.
    #[error("{0}")]
    UnexpectedStatus(UnexpectedResponseError),
    /// Invalid request.
    #[error("{0}")]
    InvalidRequest(String),
    /// Multiple registered tools share the same effective name.
    #[error("duplicate tool: {0}")]
    ToolCollision(String),
    /// Invalid image.
    #[error("Image poisoning")]
    InvalidImageRequest(),
    #[error("Selected model is at capacity. Please try a different model.")]
    ServerOverloaded,
    #[error("{message}")]
    CyberPolicy { message: String },
    #[error("{message}")]
    MisalignmentPolicyViolation {
        message: String,
        misalignment: Option<MisalignmentErrorDetails>,
    },
    #[error("{0}")]
    ResponseStreamFailed(ResponseStreamFailed),
    #[error("{0}")]
    ConnectionFailed(ConnectionFailedError),
    #[error("Quota exceeded. Check your plan and billing details.")]
    QuotaExceeded,
    #[error("We’re currently experiencing high demand, which may cause temporary errors.")]
    InternalServerError,
    /// Retry limit exceeded.
    #[error("{0}")]
    RetryLimit(RetryLimitReachedError),
    /// Agent loop died unexpectedly
    #[error("internal error; agent loop died unexpectedly")]
    InternalAgentDied,
    /// Sandbox error
    #[error("sandbox error: {0}")]
    Sandbox(#[from] SandboxErr),
    #[error("codex-linux-sandbox was required but not provided")]
    LandlockSandboxExecutableNotProvided,
    #[error("unsupported operation: {0}")]
    UnsupportedOperation(String),
    #[error("{0}")]
    RefreshTokenFailed(RefreshTokenFailedError),
    #[error("Fatal error: {0}")]
    Fatal(String),
    // -----------------------------------------------------------------
    // Automatic conversions for common external error types
    // -----------------------------------------------------------------
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[cfg(target_os = "linux")]
    #[error(transparent)]
    LandlockRuleset(#[from] landlock::RulesetError),
    #[cfg(target_os = "linux")]
    #[error(transparent)]
    LandlockPathFd(#[from] landlock::PathFdError),
    #[error(transparent)]
    TokioJoin(#[from] JoinError),
    #[error("{0}")]
    EnvVar(EnvVarError),
}

impl From<&GenexErr> for GenexErrKind {
    fn from(error: &GenexErr) -> Self {
        error.details().into()
    }
}

impl fmt::Debug for GenexErr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.details {
            GenexErrorDetails::Stream(message) => formatter
                .debug_tuple("Stream")
                .field(message)
                .field(&self.retry_delay)
                .finish(),
            details => fmt::Debug::fmt(details, formatter),
        }
    }
}

impl fmt::Display for GenexErr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.details, formatter)
    }
}

impl std::error::Error for GenexErr {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.details.source()
    }
}

impl From<GenexErrorDetails> for GenexErr {
    fn from(details: GenexErrorDetails) -> Self {
        Self {
            details,
            retry_delay: None,
        }
    }
}

impl From<CancelErr> for GenexErr {
    fn from(error: CancelErr) -> Self {
        GenexErrorDetails::from(error).into()
    }
}

impl From<SandboxErr> for GenexErr {
    fn from(error: SandboxErr) -> Self {
        GenexErrorDetails::from(error).into()
    }
}

impl From<io::Error> for GenexErr {
    fn from(error: io::Error) -> Self {
        GenexErrorDetails::from(error).into()
    }
}

impl From<serde_json::Error> for GenexErr {
    fn from(error: serde_json::Error) -> Self {
        GenexErrorDetails::from(error).into()
    }
}

impl From<JoinError> for GenexErr {
    fn from(error: JoinError) -> Self {
        GenexErrorDetails::from(error).into()
    }
}

#[cfg(target_os = "linux")]
impl From<landlock::RulesetError> for GenexErr {
    fn from(error: landlock::RulesetError) -> Self {
        GenexErrorDetails::from(error).into()
    }
}

#[cfg(target_os = "linux")]
impl From<landlock::PathFdError> for GenexErr {
    fn from(error: landlock::PathFdError) -> Self {
        GenexErrorDetails::from(error).into()
    }
}

impl From<CancelErr> for GenexErrorDetails {
    fn from(_: CancelErr) -> Self {
        GenexErrorDetails::TurnAborted
    }
}

// TODO(anp): Remove this compatibility macro once callers construct
// `GenexErrorDetails` directly.
macro_rules! genex_err_unit_constructors {
    ($($variant:ident),* $(,)?) => {
        $(
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            pub const $variant: Self = Self {
                details: GenexErrorDetails::$variant,
                retry_delay: None,
            };
        )*
    };
}

// TODO(anp): Remove this compatibility macro once callers construct
// `GenexErrorDetails` directly.
macro_rules! genex_err_tuple_constructors {
    ($($(#[$attr:meta])* $variant:ident($value:ident: $value_type:ty)),* $(,)?) => {
        $(
            $(#[$attr])*
            #[doc(hidden)]
            #[allow(non_snake_case)]
            pub fn $variant($value: $value_type) -> Self {
                GenexErrorDetails::$variant($value).into()
            }
        )*
    };
}

impl GenexErr {
    genex_err_unit_constructors!(
        TurnAborted,
        SessionBudgetExceeded,
        ContextWindowExceeded,
        SessionConfiguredNotFirstEvent,
        Timeout,
        RequestTimeout,
        Spawn,
        Interrupted,
        ServerOverloaded,
        QuotaExceeded,
        InternalServerError,
        InternalAgentDied,
        LandlockSandboxExecutableNotProvided,
    );

    genex_err_tuple_constructors!(
        Stream(message: String),
        ThreadNotFound(thread_id: ThreadId),
        UnexpectedStatus(error: UnexpectedResponseError),
        InvalidRequest(message: String),
        ResponseStreamFailed(error: ResponseStreamFailed),
        ConnectionFailed(error: ConnectionFailedError),
        RetryLimit(error: RetryLimitReachedError),
        Sandbox(error: SandboxErr),
        UnsupportedOperation(message: String),
        RefreshTokenFailed(error: RefreshTokenFailedError),
        Fatal(message: String),
        Io(error: io::Error),
        Json(error: serde_json::Error),
        #[cfg(target_os = "linux")]
        LandlockRuleset(error: landlock::RulesetError),
        #[cfg(target_os = "linux")]
        LandlockPathFd(error: landlock::PathFdError),
        TokioJoin(error: JoinError),
        EnvVar(error: EnvVarError),
    );

    // TODO(anp): Remove this compatibility constructor once callers construct
    // `GenexErrorDetails` directly.
    #[doc(hidden)]
    #[allow(non_snake_case)]
    pub fn InvalidImageRequest() -> Self {
        GenexErrorDetails::InvalidImageRequest().into()
    }

    /// Creates an error with no server-provided retry delay.
    pub fn new(details: GenexErrorDetails) -> Self {
        details.into()
    }

    /// Returns the semantic failure and its diagnostic payload.
    pub fn details(&self) -> &GenexErrorDetails {
        &self.details
    }

    pub fn is_retryable(&self) -> bool {
        match self.details() {
            GenexErrorDetails::TurnAborted
            | GenexErrorDetails::SessionBudgetExceeded
            | GenexErrorDetails::Interrupted
            | GenexErrorDetails::EnvVar(_)
            | GenexErrorDetails::Fatal(_)
            | GenexErrorDetails::QuotaExceeded
            | GenexErrorDetails::InvalidImageRequest()
            | GenexErrorDetails::InvalidRequest(_)
            | GenexErrorDetails::ToolCollision(_)
            | GenexErrorDetails::RefreshTokenFailed(_)
            | GenexErrorDetails::UnsupportedOperation(_)
            | GenexErrorDetails::Sandbox(_)
            | GenexErrorDetails::LandlockSandboxExecutableNotProvided
            | GenexErrorDetails::RetryLimit(_)
            | GenexErrorDetails::ContextWindowExceeded
            | GenexErrorDetails::ThreadNotFound(_)
            | GenexErrorDetails::AgentLimitReached { .. }
            | GenexErrorDetails::Spawn
            | GenexErrorDetails::SessionConfiguredNotFirstEvent
            | GenexErrorDetails::ServerOverloaded
            | GenexErrorDetails::CyberPolicy { .. }
            | GenexErrorDetails::MisalignmentPolicyViolation { .. } => false,
            GenexErrorDetails::Stream(..)
            | GenexErrorDetails::RateLimitExceeded(_)
            | GenexErrorDetails::Timeout
            | GenexErrorDetails::RequestTimeout
            | GenexErrorDetails::UnexpectedStatus(_)
            | GenexErrorDetails::ResponseStreamFailed(_)
            | GenexErrorDetails::ConnectionFailed(_)
            | GenexErrorDetails::InternalServerError
            | GenexErrorDetails::InternalAgentDied
            | GenexErrorDetails::Io(_)
            | GenexErrorDetails::Json(_)
            | GenexErrorDetails::TokioJoin(_) => true,
            #[cfg(target_os = "linux")]
            GenexErrorDetails::LandlockRuleset(_) | GenexErrorDetails::LandlockPathFd(_) => false,
        }
    }

    pub fn retry_delay(&self) -> Option<Duration> {
        self.retry_delay
    }

    pub fn with_retry_delay(mut self, retry_delay: Duration) -> Self {
        self.retry_delay = Some(retry_delay);
        self
    }

    /// Translate core error to client-facing protocol error.
    pub fn to_genex_protocol_error(&self) -> GenexErrorInfo {
        match &self.details {
            GenexErrorDetails::ContextWindowExceeded => GenexErrorInfo::ContextWindowExceeded,
            GenexErrorDetails::SessionBudgetExceeded => GenexErrorInfo::SessionBudgetExceeded,
            GenexErrorDetails::RateLimitExceeded(_) => GenexErrorInfo::RateLimitExceeded,
            GenexErrorDetails::QuotaExceeded => GenexErrorInfo::UsageLimitExceeded,
            GenexErrorDetails::ServerOverloaded => GenexErrorInfo::ServerOverloaded,
            GenexErrorDetails::CyberPolicy { .. } => GenexErrorInfo::CyberPolicy,
            GenexErrorDetails::MisalignmentPolicyViolation { .. } => {
                GenexErrorInfo::MisalignmentPolicyViolation
            }
            GenexErrorDetails::RetryLimit(_) => GenexErrorInfo::ResponseTooManyFailedAttempts {
                http_status_code: self.http_status_code_value(),
            },
            GenexErrorDetails::ConnectionFailed(_) => GenexErrorInfo::HttpConnectionFailed {
                http_status_code: self.http_status_code_value(),
            },
            GenexErrorDetails::ResponseStreamFailed(_) => {
                GenexErrorInfo::ResponseStreamConnectionFailed {
                    http_status_code: self.http_status_code_value(),
                }
            }
            GenexErrorDetails::RefreshTokenFailed(_) => GenexErrorInfo::Unauthorized,
            GenexErrorDetails::SessionConfiguredNotFirstEvent
            | GenexErrorDetails::InternalServerError
            | GenexErrorDetails::InternalAgentDied => GenexErrorInfo::InternalServerError,
            GenexErrorDetails::UnsupportedOperation(_)
            | GenexErrorDetails::ThreadNotFound(_)
            | GenexErrorDetails::AgentLimitReached { .. } => GenexErrorInfo::BadRequest,
            GenexErrorDetails::Sandbox(_) => GenexErrorInfo::SandboxError,
            _ => GenexErrorInfo::Other,
        }
    }

    pub fn to_error_event(&self, message_prefix: Option<String>) -> ErrorEvent {
        let error_message = self.to_string();
        let message: String = match message_prefix {
            Some(prefix) => format!("{prefix}: {error_message}"),
            None => error_message,
        };
        ErrorEvent {
            message,
            genex_error_info: Some(self.to_genex_protocol_error()),
            misalignment: match &self.details {
                GenexErrorDetails::MisalignmentPolicyViolation { misalignment, .. } => {
                    misalignment.clone()
                }
                _ => None,
            },
        }
    }

    pub fn http_status_code_value(&self) -> Option<u16> {
        let http_status_code = match &self.details {
            GenexErrorDetails::RetryLimit(err) => Some(err.status),
            GenexErrorDetails::UnexpectedStatus(err) => Some(err.status),
            GenexErrorDetails::ConnectionFailed(err) => err.source.status(),
            GenexErrorDetails::ResponseStreamFailed(err) => err.source.status(),
            _ => None,
        };
        http_status_code.as_ref().map(StatusCode::as_u16)
    }
}

#[derive(Debug)]
pub struct ConnectionFailedError {
    pub source: HttpError,
}

impl std::fmt::Display for ConnectionFailedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Connection failed: {}", self.source)
    }
}

#[derive(Debug)]
pub struct ResponseStreamFailed {
    pub source: HttpError,
    pub request_id: Option<String>,
}

impl std::fmt::Display for ResponseStreamFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error while reading the server response: {}{}",
            self.source,
            self.request_id
                .as_ref()
                .map(|id| format!(", request id: {id}"))
                .unwrap_or_default()
        )
    }
}

#[derive(Clone, Debug)]
pub struct UnexpectedResponseError {
    pub status: StatusCode,
    pub body: String,
    pub user_message: Option<String>,
    pub url: Option<String>,
    pub cf_ray: Option<String>,
    pub request_id: Option<String>,
}

const UNEXPECTED_RESPONSE_BODY_MAX_BYTES: usize = 1000;

impl UnexpectedResponseError {
    fn display_body(&self) -> String {
        if let Some(message) = self.extract_error_message() {
            return message;
        }

        let trimmed_body = self.body.trim();
        if trimmed_body.is_empty() {
            return "Unknown error".to_string();
        }

        truncate_with_ellipsis(trimmed_body, UNEXPECTED_RESPONSE_BODY_MAX_BYTES)
    }

    fn extract_error_message(&self) -> Option<String> {
        let json = serde_json::from_str::<serde_json::Value>(&self.body).ok()?;
        let message = json
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(serde_json::Value::as_str)?;
        let message = message.trim();
        if message.is_empty() {
            None
        } else {
            Some(message.to_string())
        }
    }
}

impl std::fmt::Display for UnexpectedResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut message = if let Some(user_message) = &self.user_message {
            user_message.clone()
        } else {
            let status = self.status;
            let body = self.display_body();
            format!("unexpected status {status}: {body}")
        };
        if let Some(url) = &self.url {
            message.push_str(&format!(", url: {url}"));
        }
        if let Some(cf_ray) = &self.cf_ray {
            message.push_str(&format!(", cf-ray: {cf_ray}"));
        }
        if let Some(id) = &self.request_id {
            message.push_str(&format!(", request id: {id}"));
        }
        write!(f, "{message}")
    }
}

impl std::error::Error for UnexpectedResponseError {}

fn truncate_with_ellipsis(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }

    let mut cut = max_bytes;
    while !text.is_char_boundary(cut) {
        cut = cut.saturating_sub(1);
    }
    let mut truncated = text[..cut].to_string();
    truncated.push_str("...");
    truncated
}

fn truncate_text(content: &str, policy: TruncationPolicy) -> String {
    match policy {
        TruncationPolicy::Bytes(bytes) => truncate_middle_chars(content, bytes),
        TruncationPolicy::Tokens(tokens) => truncate_middle_with_token_budget(content, tokens).0,
    }
}

#[derive(Debug)]
pub struct RetryLimitReachedError {
    pub status: StatusCode,
    pub request_id: Option<String>,
}

impl std::fmt::Display for RetryLimitReachedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exceeded retry limit, last status: {}{}",
            self.status,
            self.request_id
                .as_ref()
                .map(|id| format!(", request id: {id}"))
                .unwrap_or_default()
        )
    }
}

#[derive(Debug)]
pub struct EnvVarError {
    /// Name of the environment variable that is missing.
    pub var: String,
    /// Optional instructions to help the user get a valid value for the
    /// variable and set it.
    pub instructions: Option<String>,
}

impl std::fmt::Display for EnvVarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Missing environment variable: `{}`.", self.var)?;
        if let Some(instructions) = &self.instructions {
            write!(f, " {instructions}")?;
        }
        Ok(())
    }
}

pub fn get_error_message_ui(e: &GenexErr) -> String {
    let message = match e.details() {
        GenexErrorDetails::Sandbox(SandboxErr::Denied { output, .. }) => {
            let aggregated = output.aggregated_output.text.trim();
            if !aggregated.is_empty() {
                output.aggregated_output.text.clone()
            } else {
                let stderr = output.stderr.text.trim();
                let stdout = output.stdout.text.trim();
                match (stderr.is_empty(), stdout.is_empty()) {
                    (false, false) => format!("{stderr}\n{stdout}"),
                    (false, true) => output.stderr.text.clone(),
                    (true, false) => output.stdout.text.clone(),
                    (true, true) => format!(
                        "command failed inside sandbox with exit code {}",
                        output.exit_code
                    ),
                }
            }
        }
        // Timeouts are not sandbox errors from a UX perspective; present them plainly.
        GenexErrorDetails::Sandbox(SandboxErr::Timeout { output }) => {
            format!(
                "error: command timed out after {} ms",
                output.duration.as_millis()
            )
        }
        _ => e.to_string(),
    };

    truncate_text(
        &message,
        TruncationPolicy::Bytes(ERROR_MESSAGE_UI_MAX_BYTES),
    )
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
