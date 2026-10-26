use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
#[error(
    "failed to initialize sqlite local db at {}: {detail}",
    database_path.display()
)]
pub struct LocalStateDbStartupError {
    database_path: PathBuf,
    detail: String,
    corruption: bool,
}

impl LocalStateDbStartupError {
    pub fn new(database_path: PathBuf, detail: String) -> Self {
        Self::new_with_corruption(database_path, detail, false)
    }

    pub fn new_with_corruption(database_path: PathBuf, detail: String, corruption: bool) -> Self {
        Self {
            database_path,
            detail,
            corruption,
        }
    }

    pub fn database_path(&self) -> &Path {
        self.database_path.as_path()
    }

    pub fn detail(&self) -> &str {
        self.detail.as_str()
    }

    pub fn is_corruption(&self) -> bool {
        self.corruption
    }
}
