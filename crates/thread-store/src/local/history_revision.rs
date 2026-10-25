//! Validates local rollout snapshots before reusing them for resume.

use std::path::Path;

/// Returns an opaque revision for the exact existing rollout representation.
///
/// A missing or changing revision disables reuse. Canonicalizing the selected path prevents a
/// symlink alias from accidentally validating a different rollout after it is replaced.
pub(super) async fn read(path: &Path) -> Option<String> {
    let path = ruxex_rollout::existing_rollout_path(path).await?;
    let path = tokio::fs::canonicalize(path).await.ok()?;
    let metadata = tokio::fs::metadata(&path).await.ok()?;
    let modified = metadata.modified().ok()?;
    let created = metadata.created().ok();
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        Some((
            metadata.dev(),
            metadata.ino(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        ))
    };
    #[cfg(not(unix))]
    let identity: Option<(u64, u64, i64, i64)> = None;
    serde_json::to_string(&("local", path, metadata.len(), modified, created, identity)).ok()
}
