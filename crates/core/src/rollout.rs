use crate::config::Config;
pub use ruxex_rollout::ARCHIVED_SESSIONS_SUBDIR;
pub use ruxex_rollout::Cursor;
pub use ruxex_rollout::INTERACTIVE_SESSION_SOURCES;
pub use ruxex_rollout::RolloutRecorder;
pub use ruxex_rollout::RolloutRecorderParams;
pub use ruxex_rollout::SESSIONS_SUBDIR;
pub use ruxex_rollout::SessionMeta;
pub use ruxex_rollout::SortDirection;
pub use ruxex_rollout::ThreadItem;
pub use ruxex_rollout::ThreadSortKey;
pub use ruxex_rollout::ThreadsPage;
pub use ruxex_rollout::append_thread_name;
pub use ruxex_rollout::find_archived_thread_path_by_id_str;
pub use ruxex_rollout::find_thread_name_by_id;
pub use ruxex_rollout::find_thread_names_by_ids;
pub use ruxex_rollout::find_thread_path_by_id_str;
pub use ruxex_rollout::parse_cursor;
pub use ruxex_rollout::read_head_for_summary;
pub use ruxex_rollout::read_session_meta_line;
pub use ruxex_rollout::rollout_date_parts;

impl ruxex_rollout::RolloutConfigView for Config {
    fn ruxex_home(&self) -> &std::path::Path {
        self.ruxex_home.as_path()
    }

    fn sqlite_config(&self) -> &ruxex_state::SqliteConfig {
        self.sqlite_config()
    }

    fn cwd(&self) -> &std::path::Path {
        self.cwd.as_path()
    }
}

pub(crate) mod list {
    pub use ruxex_rollout::find_thread_path_by_id_str;
}

#[cfg(test)]
pub(crate) mod recorder {
    pub use ruxex_rollout::RolloutRecorder;
}

pub(crate) use crate::session_rollout_init_error::map_session_init_error;

pub(crate) mod truncation {
    pub(crate) use crate::thread_rollout_truncation::*;
}
