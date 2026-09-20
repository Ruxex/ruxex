use std::path::{Path, PathBuf};

pub const OPENAI_PLUGINS_GIT_URL: &str = "https://github.com/openai/plugins.git";

pub fn curated_plugins_repo_path(codex_home: &Path) -> PathBuf {
    codex_home.join(".tmp/plugins")
}

pub fn curated_plugins_api_marketplace_path(codex_home: &Path) -> PathBuf {
    curated_plugins_repo_path(codex_home).join(".agents/plugins/api_marketplace.json")
}

pub fn read_curated_plugins_sha(codex_home: &Path) -> Option<String> {
    std::fs::read_to_string(codex_home.join(".tmp/plugins.sha"))
        .ok()
        .map(|sha| sha.trim().to_string())
        .filter(|sha| !sha.is_empty())
}
