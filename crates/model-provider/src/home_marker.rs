use std::fs;
use std::io;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use crate::contract::ProviderDescriptor;
use uuid::Uuid;

/// File written at the root of a provider-scoped Ruxex home.
pub const PROVIDER_HOME_MARKER_FILE: &str = "ruxex-home.json";

/// Version of the marker file format.
pub const PROVIDER_HOME_FORMAT_VERSION: u32 = 2;

/// Non-secret identity stored in a provider-scoped Ruxex home.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderHomeMarker {
    #[serde(rename = "formatVersion")]
    pub format_version: u32,
    #[serde(rename = "providerKind")]
    pub provider_kind: String,
    #[serde(rename = "providerProtocolVersion")]
    pub provider_protocol_version: String,
    #[serde(rename = "serverSignature")]
    pub server_signature: String,
}

impl From<&ProviderDescriptor> for ProviderHomeMarker {
    fn from(descriptor: &ProviderDescriptor) -> Self {
        Self {
            format_version: PROVIDER_HOME_FORMAT_VERSION,
            provider_kind: descriptor.provider_kind.clone(),
            provider_protocol_version: descriptor.provider_protocol_version.clone(),
            server_signature: Uuid::new_v4().to_string(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderHomeError {
    #[error("failed to access provider data root `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(
        "provider data root `{path}` has no marker `{marker_file}`; explicit initialization or migration is required"
    )]
    MissingMarker { path: PathBuf, marker_file: PathBuf },
    #[error("invalid marker `{marker_file}` in `{path}`: {source}")]
    InvalidMarker {
        path: PathBuf,
        marker_file: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "provider data root `{path}` has an incompatible marker: expected {expected:?}, found {actual:?}"
    )]
    DescriptorMismatch {
        path: PathBuf,
        expected: ProviderHomeMarker,
        actual: ProviderHomeMarker,
    },
    #[error("failed to serialize marker `{marker_file}` for `{path}`: {source}")]
    Serialize {
        path: PathBuf,
        marker_file: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("provider home marker must be a single filename, got `{marker_file}`")]
    InvalidMarkerFile { marker_file: PathBuf },
}

/// Initializes or validates a provider-scoped data root without opening any state or credentials.
pub fn mount_provider_home(
    path: impl AsRef<Path>,
    descriptor: &ProviderDescriptor,
) -> Result<ProviderHomeMarker, ProviderHomeError> {
    mount_provider_home_with_marker(path, PROVIDER_HOME_MARKER_FILE, descriptor)
}

/// Initializes or validates a provider-scoped data root using a product-specific marker.
pub fn mount_provider_home_with_marker(
    path: impl AsRef<Path>,
    marker_file: impl AsRef<Path>,
    descriptor: &ProviderDescriptor,
) -> Result<ProviderHomeMarker, ProviderHomeError> {
    let path = path.as_ref().to_path_buf();
    let marker_file = validate_marker_file(marker_file.as_ref())?;
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|source| ProviderHomeError::Io {
            path: path.clone(),
            source,
        })?;
        return write_marker(&path, &marker_file, descriptor);
    }

    let metadata = fs::metadata(&path).map_err(|source| ProviderHomeError::Io {
        path: path.clone(),
        source,
    })?;
    if !metadata.is_dir() {
        return Err(ProviderHomeError::Io {
            path,
            source: io::Error::new(
                io::ErrorKind::NotADirectory,
                "provider data root is not a directory",
            ),
        });
    }

    let marker_path = path.join(&marker_file);
    match fs::read_to_string(&marker_path) {
        Ok(contents) => {
            let actual =
                serde_json::from_str::<ProviderHomeMarker>(&contents).map_err(|source| {
                    ProviderHomeError::InvalidMarker {
                        path: marker_path.clone(),
                        marker_file: marker_file.clone(),
                        source,
                    }
                })?;
            let expected = ProviderHomeMarker {
                format_version: PROVIDER_HOME_FORMAT_VERSION,
                provider_kind: descriptor.provider_kind.clone(),
                provider_protocol_version: descriptor.provider_protocol_version.clone(),
                server_signature: actual.server_signature.clone(),
            };
            if actual != expected {
                return Err(ProviderHomeError::DescriptorMismatch {
                    path,
                    expected,
                    actual,
                });
            }
            Ok(actual)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let mut entries = fs::read_dir(&path).map_err(|source| ProviderHomeError::Io {
                path: path.clone(),
                source,
            })?;
            let has_existing_data = entries
                .next()
                .transpose()
                .map_err(|source| ProviderHomeError::Io {
                    path: path.clone(),
                    source,
                })?
                .is_some();
            if has_existing_data {
                return Err(ProviderHomeError::MissingMarker { path, marker_file });
            }
            write_marker(&path, &marker_file, descriptor)
        }
        Err(source) => Err(ProviderHomeError::Io {
            path: marker_path,
            source,
        }),
    }
}

fn write_marker(
    path: &Path,
    marker_file: &Path,
    descriptor: &ProviderDescriptor,
) -> Result<ProviderHomeMarker, ProviderHomeError> {
    let marker = ProviderHomeMarker::from(descriptor);
    let contents =
        serde_json::to_string_pretty(&marker).map_err(|source| ProviderHomeError::Serialize {
            path: path.to_path_buf(),
            marker_file: marker_file.to_path_buf(),
            source,
        })?;
    fs::write(path.join(marker_file), format!("{contents}\n")).map_err(|source| {
        ProviderHomeError::Io {
            path: path.join(marker_file),
            source,
        }
    })?;
    Ok(marker)
}

fn validate_marker_file(marker_file: &Path) -> Result<PathBuf, ProviderHomeError> {
    let mut components = marker_file.components();
    let Some(Component::Normal(component)) = components.next() else {
        return Err(ProviderHomeError::InvalidMarkerFile {
            marker_file: marker_file.to_path_buf(),
        });
    };
    if components.next().is_some() {
        return Err(ProviderHomeError::InvalidMarkerFile {
            marker_file: marker_file.to_path_buf(),
        });
    }
    Ok(PathBuf::from(component))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::CURRENT_PROVIDER_PROTOCOL_VERSION;
    use crate::ProviderDescriptor;

    fn descriptor(kind: &str) -> ProviderDescriptor {
        ProviderDescriptor::new(kind, CURRENT_PROVIDER_PROTOCOL_VERSION)
    }

    #[test]
    fn new_home_is_initialized_with_a_marker() {
        let temp = tempfile::tempdir().expect("temporary home");
        let home = temp.path().join("provider-home");

        let marker = mount_provider_home(&home, &descriptor("provider-a"))
            .expect("new home should be initialized");

        assert_eq!(marker.provider_kind, "provider-a");
        assert_eq!(marker.format_version, PROVIDER_HOME_FORMAT_VERSION);
        assert_eq!(
            fs::read_to_string(home.join(PROVIDER_HOME_MARKER_FILE))
                .expect("marker should be written"),
            serde_json::to_string_pretty(&marker).expect("marker should serialize") + "\n"
        );
    }

    #[test]
    fn empty_existing_home_is_initialized_with_a_marker() {
        let temp = tempfile::tempdir().expect("temporary home");

        let marker = mount_provider_home(temp.path(), &descriptor("provider-a"))
            .expect("empty home should be initialized");

        assert_eq!(marker.provider_kind, "provider-a");
    }

    #[test]
    fn existing_home_without_a_marker_requires_explicit_initialization() {
        let temp = tempfile::tempdir().expect("temporary home");
        fs::write(temp.path().join("state.sqlite"), b"existing state")
            .expect("state fixture should be written");

        let error = mount_provider_home(temp.path(), &descriptor("provider-a"))
            .expect_err("unmarked existing home should be rejected");

        assert!(matches!(error, ProviderHomeError::MissingMarker { .. }));
    }

    #[test]
    fn mismatched_provider_marker_is_rejected() {
        let temp = tempfile::tempdir().expect("temporary home");
        mount_provider_home(temp.path(), &descriptor("provider-a"))
            .expect("initial mount should succeed");

        let error = mount_provider_home(temp.path(), &descriptor("provider-b"))
            .expect_err("different provider should be rejected");

        assert!(matches!(
            error,
            ProviderHomeError::DescriptorMismatch { .. }
        ));
    }

    #[test]
    fn matching_provider_marker_is_reused() {
        let temp = tempfile::tempdir().expect("temporary home");
        let descriptor = descriptor("provider-a");
        let first = mount_provider_home(temp.path(), &descriptor).expect("initial mount");
        let second = mount_provider_home(temp.path(), &descriptor).expect("reuse mount");

        assert_eq!(first, second);
    }

    #[test]
    fn new_home_gets_a_stable_server_signature() {
        let temp = tempfile::tempdir().expect("temporary home");
        let descriptor = descriptor("provider-a");

        let first = mount_provider_home(temp.path(), &descriptor).expect("initial mount");
        let second = mount_provider_home(temp.path(), &descriptor).expect("reuse mount");

        assert!(!first.server_signature.is_empty());
        assert_eq!(first.server_signature, second.server_signature);
    }

    #[test]
    fn old_marker_format_requires_explicit_migration() {
        let temp = tempfile::tempdir().expect("temporary home");
        fs::write(
            temp.path().join(PROVIDER_HOME_MARKER_FILE),
            serde_json::json!({
                "formatVersion": 1,
                "providerKind": "provider-a",
                "providerProtocolVersion": CURRENT_PROVIDER_PROTOCOL_VERSION,
            })
            .to_string(),
        )
        .expect("old marker should be written");

        let error = mount_provider_home(temp.path(), &descriptor("provider-a"))
            .expect_err("old marker should not be opened implicitly");

        assert!(matches!(error, ProviderHomeError::InvalidMarker { .. }));
    }

    #[test]
    fn custom_marker_filename_is_scoped_to_the_provider_home() {
        let temp = tempfile::tempdir().expect("temporary home");
        let descriptor = descriptor("provider-a");

        mount_provider_home_with_marker(temp.path(), "example-home.json", &descriptor)
            .expect("custom marker should initialize");

        assert!(temp.path().join("example-home.json").is_file());
        assert!(!temp.path().join(PROVIDER_HOME_MARKER_FILE).exists());
        mount_provider_home_with_marker(temp.path(), "example-home.json", &descriptor)
            .expect("custom marker should be reusable");
    }

    #[test]
    fn custom_marker_filename_must_be_a_single_component() {
        let temp = tempfile::tempdir().expect("temporary home");
        let descriptor = descriptor("provider-a");

        let error =
            mount_provider_home_with_marker(temp.path(), "nested/example-home.json", &descriptor)
                .expect_err("nested marker paths should be rejected");

        assert!(matches!(error, ProviderHomeError::InvalidMarkerFile { .. }));
    }
}
