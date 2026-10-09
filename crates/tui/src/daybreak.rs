//! Read-only Daybreak eligibility used to choose cyber refusal copy.
//! Account-scoped discovery runs in the background; pending/failed reads use neutral copy.
//! The TUI does not configure the app’s Daybreak access program.

use crate::app_server_session::AppServerSession;
use crate::legacy_core::config::Config;
use std::sync::Arc;
use tokio::sync::OnceCell;

pub(crate) type NoticeCache = Arc<OnceCell<Notice>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Notice {
    Apply,
    Astra,
    #[default]
    Limited,
}

pub(crate) fn prefetch_notice(config: &Config, server: &AppServerSession, cache: NoticeCache) {
    let _ = config;
    if server.uses_remote_workspace() || cache.get().is_some() {
        return;
    }
    tokio::spawn(async move {
        cache.get_or_init(|| async { Notice::default() }).await;
    });
}

impl Notice {
    pub(crate) fn for_model(self, _model: &str) -> Self {
        // Model availability is provider-owned. The TUI only renders the
        // account-scoped access result and does not maintain a model catalog.
        self
    }
}
