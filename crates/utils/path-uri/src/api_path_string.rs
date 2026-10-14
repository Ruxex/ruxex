use crate::PathConvention;
use crate::PathUri;
use crate::PathUriParseError;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;
use std::fmt;
use std::path::Path;
use thiserror::Error;
use ts_rs::TS;

/// A UTF-8 path preserved at the app-server API boundary.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, TS)]
#[serde(transparent)]
#[ts(type = "string")]
pub struct LegacyAppPathString(String);

impl LegacyAppPathString {
    pub fn from_string(path: impl Into<String>) -> Self {
        Self(path.into())
    }
    pub fn from_path(path: &Path) -> Self {
        Self(path.to_string_lossy().into_owned())
    }
    pub fn from_abs_path(path: &AbsolutePathBuf) -> Self {
        Self::from_path(path.as_path())
    }

    pub fn from_path_uri(
        path: &PathUri,
        convention: PathConvention,
    ) -> Result<Self, LegacyAppPathStringError> {
        let PathConvention::Posix = convention;
        if let Some(path_bytes) = path.opaque_fallback_bytes() {
            if !path_bytes.starts_with(b"/") {
                return Err(LegacyAppPathStringError::OpaqueFallback {
                    path: path.to_string(),
                });
            }
            return Ok(Self(String::from_utf8_lossy(&path_bytes).into_owned()));
        }
        let url = path.to_url();
        if url.host_str().is_some() {
            return Err(incompatible_convention(path, convention));
        }
        let mut rendered = String::new();
        let segments = url
            .path_segments()
            .ok_or_else(|| incompatible_convention(path, convention))?;
        for segment in segments {
            rendered.push('/');
            rendered.push_str(&decode_native_segment(segment));
        }
        Ok(Self(rendered))
    }

    pub fn to_path_uri(
        &self,
        convention: PathConvention,
    ) -> Result<PathUri, LegacyAppPathStringError> {
        PathUri::from_absolute_native_path(&self.0, convention).ok_or_else(|| {
            LegacyAppPathStringError::InvalidNativePath {
                path: self.0.clone(),
                convention: Some(convention),
            }
        })
    }

    pub fn resolve_against(
        &self,
        cwd: &PathUri,
        user_home_dir: Option<&PathUri>,
    ) -> Result<PathUri, LegacyAppPathStringError> {
        let convention = cwd.infer_path_convention().ok_or_else(|| {
            LegacyAppPathStringError::MissingBaseConvention {
                cwd: cwd.to_string(),
            }
        })?;
        if let Some(suffix) = convention.home_relative_suffix(&self.0) {
            let home =
                user_home_dir.ok_or_else(|| LegacyAppPathStringError::MissingHomeDirectory {
                    path: self.0.clone(),
                })?;
            return Ok(home.join(suffix.trim_start_matches('/'))?);
        }
        match self.infer_absolute_path_convention() {
            Some(path_convention) if path_convention == convention => self.to_path_uri(convention),
            Some(path_convention) => Err(LegacyAppPathStringError::MismatchedConvention {
                path: self.0.clone(),
                path_convention,
                cwd: cwd.to_string(),
                convention,
            }),
            None => Ok(cwd.join(&self.0)?),
        }
    }

    pub fn to_inferred_path_uri(&self) -> Option<PathUri> {
        PathUri::try_from(self.clone()).ok()
    }
    pub fn render_for_ui(&self) -> String {
        self.to_inferred_path_uri()
            .map(|path| path.inferred_native_path_string())
            .unwrap_or_else(|| self.0.clone())
    }
    pub fn to_inferred_abs_path(&self) -> Option<AbsolutePathBuf> {
        AbsolutePathBuf::try_from(self.clone()).ok()
    }
    pub fn infer_absolute_path_convention(&self) -> Option<PathConvention> {
        self.0.starts_with('/').then_some(PathConvention::Posix)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<AbsolutePathBuf> for LegacyAppPathString {
    fn from(path: AbsolutePathBuf) -> Self {
        Self::from_abs_path(&path)
    }
}
impl From<PathUri> for LegacyAppPathString {
    fn from(path: PathUri) -> Self {
        Self(path.inferred_native_path_string())
    }
}

impl TryFrom<LegacyAppPathString> for PathUri {
    type Error = LegacyAppPathStringError;
    fn try_from(path: LegacyAppPathString) -> Result<Self, Self::Error> {
        let Some(convention) = path.infer_absolute_path_convention() else {
            return Err(LegacyAppPathStringError::InvalidNativePath {
                path: path.0,
                convention: None,
            });
        };
        path.to_path_uri(convention)
    }
}

impl TryFrom<LegacyAppPathString> for AbsolutePathBuf {
    type Error = LegacyAppPathStringError;
    fn try_from(path: LegacyAppPathString) -> Result<Self, Self::Error> {
        AbsolutePathBuf::from_absolute_path_checked(path.as_str()).map_err(|_| {
            LegacyAppPathStringError::InvalidNativePath {
                path: path.0,
                convention: None,
            }
        })
    }
}

impl fmt::Display for LegacyAppPathString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Serialize for LegacyAppPathString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}
impl JsonSchema for LegacyAppPathString {
    fn schema_name() -> String {
        "LegacyAppPathString".to_string()
    }
    fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(generator)
    }
}

fn decode_native_segment(segment: &str) -> String {
    String::from_utf8_lossy(&urlencoding::decode_binary(segment.as_bytes())).into_owned()
}
fn incompatible_convention(path: &PathUri, convention: PathConvention) -> LegacyAppPathStringError {
    LegacyAppPathStringError::IncompatibleConvention {
        path: path.to_string(),
        convention,
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LegacyAppPathStringError {
    #[error("opaque fallback path URI `{path}` cannot be recovered as a native path")]
    OpaqueFallback { path: String },
    #[error("path URI `{path}` cannot be rendered using {convention} path syntax")]
    IncompatibleConvention {
        path: String,
        convention: PathConvention,
    },
    #[error("path `{path}` is not absolute{convention}", convention = .convention.map(|convention| format!(" using {convention} path syntax")).unwrap_or_default())]
    InvalidNativePath {
        path: String,
        convention: Option<PathConvention>,
    },
    #[error("unsupported configuration path {path:?} using {convention} path syntax")]
    UnsupportedConfigPath {
        path: String,
        convention: PathConvention,
    },
    #[error("path URI `{cwd}` has no path convention")]
    MissingBaseConvention { cwd: String },
    #[error("cannot resolve home-relative path `{path}` without an executor home")]
    MissingHomeDirectory { path: String },
    #[error(
        "path {path} uses {path_convention} paths, but executor cwd {cwd} uses {convention} paths"
    )]
    MismatchedConvention {
        path: String,
        path_convention: PathConvention,
        cwd: String,
        convention: PathConvention,
    },
    #[error(transparent)]
    PathUri(#[from] PathUriParseError),
}

#[cfg(test)]
#[path = "api_path_string_tests.rs"]
mod tests;
