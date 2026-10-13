//! Cancels device ceremonies when cached authentication changes.

use genex_model_provider::SharedModelProvider;

use super::*;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Identity {
    user_id: String,
    account_id: String,
}

impl OutgoingMessageSender {
    /// Trusted host activation enables a UI integration after its signing path is available.
    pub(crate) async fn enable_user_verification_connection(&self, connection_id: ConnectionId) {
        self.verification_connections
            .lock()
            .await
            .insert(connection_id);
    }

    /// Stop routing ceremonies immediately, without waiting for unrelated RPCs to drain.
    pub(crate) async fn disconnect_user_verification_connection(
        &self,
        connection_id: ConnectionId,
    ) {
        self.verification_connections
            .lock()
            .await
            .remove(&connection_id);
        self.request_id_to_callback
            .lock()
            .await
            .retain(|_, entry| entry.verification_owner != Some(connection_id));
    }

    pub(crate) fn watch_user_verification_auth(self: &Arc<Self>, provider: SharedModelProvider) {
        let _ = self.verification_auth.set(provider);
    }

    pub(super) fn verification_identity(&self) -> Option<Identity> {
        None
    }

    pub(super) fn verification_auth_revision(&self) -> Option<u64> {
        self.verification_auth
            .get()
            .and_then(|provider| provider.auth_state().revision)
    }
}
