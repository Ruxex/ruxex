//! Deterministic AppServer implementation for end-to-end tests.

use ruxex_app_server_protocol::{
    JSONRPCError, JSONRPCErrorError, JSONRPCMessage, JSONRPCRequest, JSONRPCResponse, RequestId,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io, path::PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};

const MOCK_SIGNATURE: &str = "ruxex-app-server-mock";
const MOCK_PROVIDER_PROTOCOL_VERSION: &str = "mock";
const MOCK_BUILD_VERSION: &str = "ruxex-app-server-mock/0.1.0";

/// Builder for a [`MockAppServer`].
#[derive(Debug, Default)]
pub struct MockAppServerBuilder {
    responses: BTreeMap<String, Value>,
    ruxex_home: Option<PathBuf>,
    identity_signature: Option<String>,
    provider_protocol_version: Option<String>,
}

impl MockAppServerBuilder {
    /// Configures a JSON response fixture for a request method.
    pub fn with_response(mut self, method: impl Into<String>, response: Value) -> Self {
        self.responses.insert(method.into(), response);
        self
    }

    /// Configures the mock home reported by `initialize`.
    pub fn with_ruxex_home(mut self, path: impl Into<PathBuf>) -> Self {
        self.ruxex_home = Some(path.into());
        self
    }

    /// Configures the signature reported by `initialize`.
    pub fn with_identity_signature(mut self, signature: impl Into<String>) -> Self {
        self.identity_signature = Some(signature.into());
        self
    }

    /// Configures the provider protocol version reported by `initialize`.
    pub fn with_provider_protocol_version(mut self, version: impl Into<String>) -> Self {
        self.provider_protocol_version = Some(version.into());
        self
    }

    /// Builds a mock server.
    ///
    /// The default reported mock home is the current working directory. Use
    /// [`Self::with_ruxex_home`] when a test needs a fixed path.
    pub fn build(self) -> io::Result<MockAppServer> {
        let ruxex_home = match self.ruxex_home {
            Some(path) => path,
            None => std::env::current_dir()?,
        };

        Ok(MockAppServer {
            responses: self.responses,
            ruxex_home,
            identity_signature: self
                .identity_signature
                .unwrap_or_else(|| MOCK_SIGNATURE.to_owned()),
            provider_protocol_version: self
                .provider_protocol_version
                .unwrap_or_else(|| MOCK_PROVIDER_PROTOCOL_VERSION.to_owned()),
        })
    }
}

/// A deterministic AppServer substitute intended for E2E test harnesses.
#[derive(Clone, Debug)]
pub struct MockAppServer {
    responses: BTreeMap<String, Value>,
    ruxex_home: PathBuf,
    identity_signature: String,
    provider_protocol_version: String,
}

impl MockAppServer {
    /// Starts building a [`MockAppServer`].
    pub fn builder() -> MockAppServerBuilder {
        MockAppServerBuilder::default()
    }

    /// Handles one JSON-RPC message without performing I/O.
    ///
    /// Requests use a configured fixture when one exists. Otherwise,
    /// `initialize` receives a valid mock initialize payload and all other
    /// methods receive a deterministic `{ "mock": true, "method": ... }`
    /// payload. Notifications and incoming responses/errors produce no output.
    pub fn handle_message(&self, message: JSONRPCMessage) -> Option<JSONRPCMessage> {
        match message {
            JSONRPCMessage::Request(request) => Some(self.handle_request(request)),
            JSONRPCMessage::Notification(_)
            | JSONRPCMessage::Response(_)
            | JSONRPCMessage::Error(_) => None,
        }
    }

    /// Runs the mock over newline-delimited JSON on stdin/stdout.
    ///
    /// stdout is reserved for JSON-RPC responses so the process can be used
    /// directly as an E2E child process. Malformed input without a recoverable
    /// request ID is reported on stderr and does not terminate the server.
    pub async fn run_stdio(&self) -> io::Result<()> {
        let stdin = tokio::io::stdin();
        let mut lines = BufReader::new(stdin).lines();
        let stdout = tokio::io::stdout();
        let mut stdout = BufWriter::new(stdout);

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<JSONRPCMessage>(&line) {
                Ok(message) => {
                    if let Some(response) = self.handle_message(message) {
                        write_message(&mut stdout, &response).await?;
                    }
                }
                Err(error) => {
                    if let Some(id) = recover_request_id(&line) {
                        let response = JSONRPCMessage::Error(JSONRPCError {
                            error: JSONRPCErrorError {
                                code: -32600,
                                data: None,
                                message: "invalid request".to_owned(),
                            },
                            id,
                        });
                        write_message(&mut stdout, &response).await?;
                    } else {
                        eprintln!("ruxex-app-server-mock: ignoring invalid input: {error}");
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_request(&self, request: JSONRPCRequest) -> JSONRPCMessage {
        let result = self
            .responses
            .get(&request.method)
            .cloned()
            .unwrap_or_else(|| {
                if request.method == "initialize" {
                    self.initialize_response()
                } else {
                    json!({"mock": true, "method": request.method})
                }
            });

        JSONRPCMessage::Response(JSONRPCResponse {
            id: request.id,
            result,
        })
    }

    fn initialize_response(&self) -> Value {
        json!({
            "userAgent": MOCK_BUILD_VERSION,
            "productName": "Ruxex",
            "productSlug": "ruxex",
            "ruxexHome": self.ruxex_home,
            "platformFamily": std::env::consts::FAMILY,
            "platformOs": std::env::consts::OS,
            "appServerIdentity": {
                "signature": self.identity_signature,
                "version": format!(
                    "app-server/1;provider/{}",
                    self.provider_protocol_version
                ),
                "buildVersion": MOCK_BUILD_VERSION,
            },
        })
    }
}

async fn write_message<W>(writer: &mut W, message: &JSONRPCMessage) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let payload = serde_json::to_vec(message).map_err(io::Error::other)?;
    writer.write_all(&payload).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await
}

fn recover_request_id(line: &str) -> Option<RequestId> {
    let value = serde_json::from_str::<Value>(line).ok()?;
    serde_json::from_value(value.get("id")?.clone()).ok()
}
