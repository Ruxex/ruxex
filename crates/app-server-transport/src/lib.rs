pub mod daemon_recovery;
/// Only managed app-server launches accept the local socket shutdown request.
pub const DAEMON_SHUTDOWN_SOCKET_ENV: &str = "RUXEX_DAEMON_SHUTDOWN_SOCKET";
mod outgoing_message;
mod transport;

pub use outgoing_message::ConnectionId;
pub use outgoing_message::OutgoingError;
pub use outgoing_message::OutgoingMessage;
pub use outgoing_message::OutgoingResponse;
pub use outgoing_message::QueuedOutgoingMessage;
pub use transport::AppServerStartupLock;
pub use transport::AppServerTransport;
pub use transport::AppServerTransportParseError;
pub use transport::CHANNEL_CAPACITY;
pub use transport::ConnectionOrigin;
pub use transport::DaemonShutdownAccess;
pub use transport::TransportEvent;
pub use transport::acquire_app_server_startup_lock;
pub use transport::app_server_control_socket_path;
pub use transport::app_server_startup_lock_path;
pub use transport::auth;
pub use transport::daemon_recovery_file_path;
pub use transport::start_control_socket_acceptor;
pub use transport::start_stdio_connection;
pub use transport::start_websocket_acceptor;
