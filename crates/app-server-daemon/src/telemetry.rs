//! Bounded observations for foreground callers. This module never exports telemetry;
//! in particular, detached child processes must not create a telemetry provider.

use std::path::Path;

/// Only presence and enabled state leave the settings reader, never values or contents.
/// Missing files mean defaults; unreadable or invalid settings remain unknown.
pub async fn settings_tags(ruxex_home: &Path) -> [(&'static str, &'static str); 1] {
    let path = ruxex_home
        .join(crate::STATE_DIR_NAME)
        .join(crate::SETTINGS_FILE_NAME);
    crate::settings::telemetry_tags(&path)
        .await
        .unwrap_or([("shutdown_grace_setting", "unknown")])
}
