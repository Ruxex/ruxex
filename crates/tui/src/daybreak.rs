//! Read-only Daybreak eligibility used to choose cyber refusal copy.
//! Account-scoped discovery runs in the background; pending/failed reads use neutral copy.
//! Astra takes precedence; the TUI does not configure the app’s Daybreak access program.

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
    if config.model_provider_id != "openai"
        || server.uses_remote_workspace()
        || cache.get().is_some()
    {
        return;
    }
    tokio::spawn(async move {
        cache.get_or_init(|| async { Notice::default() }).await;
    });
}

impl Notice {
    pub(crate) fn for_model(self, model: &str) -> Self {
        match model {
            // Same identifiers as the app's isDaybreakUnavailableModel.
            "gpt-6-astra" | "gpt-6-astra-wm" => Self::Astra,
            "gpt-5.6-sol" => self,
            // Other model/access-program mappings are not established for the TUI.
            _ => Self::Limited,
        }
    }
}

#[cfg(test)]
#[path = "daybreak_tests.rs"]
mod tests;
