//! Owns a spawned child until its initial input is accepted.

use crate::thread_manager::ThreadManagerState;
use ruxex_agent_graph_store::ThreadSpawnEdgeStatus;
use ruxex_protocol::ThreadId;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::warn;

pub(super) struct PendingSpawn {
    state: Arc<ThreadManagerState>,
    child: Option<ThreadId>,
    edge_write: Option<JoinHandle<()>>,
}

impl PendingSpawn {
    pub(super) fn new(state: Arc<ThreadManagerState>, child: ThreadId) -> Self {
        Self {
            state,
            child: Some(child),
            edge_write: None,
        }
    }

    pub(super) fn set_edge_write(&mut self, edge_write: JoinHandle<()>) {
        self.edge_write = Some(edge_write);
    }

    pub(super) async fn wait_for_edge(&mut self) {
        if let Some(edge_write) = self.edge_write.take() {
            let _ = edge_write.await;
        }
    }

    pub(super) fn disarm(mut self) {
        self.child = None;
    }
}

impl Drop for PendingSpawn {
    fn drop(&mut self) {
        let Some(child) = self.child.take() else {
            return;
        };
        eprintln!("pending spawn dropped for {child}");
        let state = Arc::clone(&self.state);
        let edge_write = self.edge_write.take();
        drop(tokio::spawn(async move {
            if let Some(thread) = state.remove_thread(&child).await {
                if let Err(error) = thread.shutdown_and_wait().await {
                    warn!("failed to stop cancelled child spawn: {error}");
                }
                if let Some(live_thread) = thread.session.live_thread()
                    && let Err(error) = live_thread.discard().await
                {
                    warn!("failed to discard cancelled child spawn: {error}");
                }
            }
            if let Some(edge_write) = edge_write {
                let _ = edge_write.await;
            }
            if let Some(store) = state.agent_graph_store()
                && let Err(error) = store
                    .set_thread_spawn_edge_status(child, ThreadSpawnEdgeStatus::Closed)
                    .await
            {
                warn!("failed to close cancelled child spawn edge: {error}");
            }
        }));
    }
}
