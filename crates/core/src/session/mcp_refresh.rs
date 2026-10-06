use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tokio::sync::AcquireError;
use tokio::sync::Semaphore;
use tokio::sync::SemaphorePermit;

/// Owns MCP invalidation and the single gate used to publish runtime updates.
pub(super) struct McpRefresh {
    pending: AtomicBool,
    gate: Semaphore,
}

impl McpRefresh {
    pub(super) fn new() -> Self {
        Self {
            pending: AtomicBool::new(false),
            gate: Semaphore::new(/*permits*/ 1),
        }
    }

    pub(super) fn invalidate(&self) {
        self.pending.store(true, Ordering::Release);
    }

    #[cfg(test)]
    pub(super) fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Acquire)
    }

    pub(super) fn claim(&self) -> bool {
        self.pending.swap(false, Ordering::AcqRel)
    }

    #[tracing::instrument(name = "mcp.runtime.refresh_wait", skip_all)]
    pub(super) async fn acquire(&self) -> Result<SemaphorePermit<'_>, AcquireError> {
        self.gate.acquire().await
    }

    pub(super) fn close(&self) {
        self.gate.close();
    }
}

/// Restores a claimed refresh when its task is cancelled before publication.
pub(super) struct McpRefreshInvalidationGuard<'a> {
    pub(super) refresh: &'a McpRefresh,
    pub(super) published: bool,
}

impl Drop for McpRefreshInvalidationGuard<'_> {
    fn drop(&mut self) {
        if !self.published {
            self.refresh.invalidate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invalidation_is_claimed_once_and_guard_restores_it() {
        let refresh = McpRefresh::new();
        assert!(!refresh.is_pending());
        assert!(!refresh.claim());

        refresh.invalidate();
        assert!(refresh.is_pending());
        assert!(refresh.claim());
        assert!(!refresh.is_pending());
        assert!(!refresh.claim());

        {
            let _permit = refresh.acquire().await.expect("gate should be open");
        }

        {
            let _guard = McpRefreshInvalidationGuard {
                refresh: &refresh,
                published: false,
            };
        }
        assert!(refresh.is_pending());
        assert!(refresh.claim());

        {
            let _guard = McpRefreshInvalidationGuard {
                refresh: &refresh,
                published: true,
            };
        }
        assert!(!refresh.is_pending());
    }

    #[tokio::test]
    async fn closed_gate_rejects_new_acquisitions() {
        let refresh = McpRefresh::new();
        refresh.close();
        assert!(refresh.acquire().await.is_err());
    }
}
