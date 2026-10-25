//! Executor platform identity, independent of the host resolving a request.

use crate::PathConvention;

/// Operating system reported by an executor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Macos,
    Windows,
    /// Missing or unrecognized metadata must not be treated as a known platform.
    #[default]
    Unknown,
}

impl Platform {
    /// Normalize executor metadata without substituting the current host platform.
    pub fn from_platform_os(platform_os: Option<&str>) -> Self {
        match platform_os {
            Some("linux") => Self::Linux,
            Some("macos") => Self::Macos,
            Some("windows") => Self::Windows,
            Some(_) | None => Self::Unknown,
        }
    }

    /// Return the platform of the current process.
    pub const fn native() -> Self {
        if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Unknown
        }
    }

    /// Return the path grammar Ruxex currently supports for this platform.
    ///
    /// Windows remains an explicit identity even though the Ruxex path-URI
    /// implementation is POSIX-only today. Keeping that distinction prevents
    /// unknown or remote executor metadata from being silently rewritten.
    pub const fn path_convention(self) -> Option<PathConvention> {
        match self {
            Self::Linux | Self::Macos => Some(PathConvention::Posix),
            Self::Windows | Self::Unknown => None,
        }
    }
}

#[cfg(test)]
#[path = "platform_tests.rs"]
mod tests;
