//! Paths resolved using the app-server host's platform rules.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppServerPath(String);

impl AppServerPath {
    pub fn from_app_server(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn from_absolute_str(raw: &str) -> Option<Self> {
        raw.starts_with('/').then(|| Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn components(&self) -> Vec<&str> {
        self.0.split('/').filter(|part| !part.is_empty()).collect()
    }

    pub fn join(&self, segment: impl AsRef<str>) -> Self {
        Self(format!(
            "{}/{}",
            self.0.trim_end_matches('/'),
            segment.as_ref()
        ))
    }
}

impl fmt::Display for AppServerPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
