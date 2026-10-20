use dirs::home_dir;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use std::path::{Path, PathBuf};

/// Build-time configurable fallback directory name under the user's home directory.
///
/// Set `RUXEX_DEFAULT_HOME_DIR_NAME` when building a provider-specific binary to avoid
/// collisions between binaries that otherwise share the same default home.
pub const DEFAULT_RUXEX_HOME_DIR_NAME: &str = match option_env!("RUXEX_DEFAULT_HOME_DIR_NAME") {
    Some(value) => value,
    None => ".ruxex",
};

/// The marker filename used by the default Ruxex product identity.
pub const DEFAULT_RUXEX_HOME_MARKER_FILE: &str = "ruxex-home.json";

/// Product-scoped identity shared by the server and clients of a built host.
///
/// The identity is intentionally separate from UI branding. It controls the
/// stable product names used for persistent data locations and protocol
/// metadata, while [`crate::DEFAULT_RUXEX_HOME_DIR_NAME`] keeps the legacy
/// Ruxex build-time override available for the default product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductIdentity {
    product_name: String,
    product_slug: String,
    home_dir_name: String,
    home_marker_file: String,
}

impl ProductIdentity {
    /// Construct the identity used by Ruxex itself.
    pub fn ruxex() -> Self {
        Self {
            product_name: "Ruxex".to_owned(),
            product_slug: "ruxex".to_owned(),
            home_dir_name: DEFAULT_RUXEX_HOME_DIR_NAME.to_owned(),
            home_marker_file: DEFAULT_RUXEX_HOME_MARKER_FILE.to_owned(),
        }
    }

    /// Construct a product identity for a host application built from Ruxex.
    ///
    /// The slug, home directory name, and marker filename are single path
    /// components so a host cannot accidentally redirect product state into a
    /// nested or parent directory.
    pub fn new(
        product_name: impl Into<String>,
        product_slug: impl Into<String>,
        home_dir_name: impl Into<String>,
        home_marker_file: impl Into<String>,
    ) -> std::io::Result<Self> {
        let product_name = product_name.into();
        let product_slug = product_slug.into();
        let home_dir_name = home_dir_name.into();
        let home_marker_file = home_marker_file.into();

        if product_name.trim().is_empty() {
            return Err(invalid_identity_component("product name", &product_name));
        }
        validate_identity_component("product slug", &product_slug)?;
        validate_identity_component("home directory name", &home_dir_name)?;
        validate_identity_component("home marker filename", &home_marker_file)?;

        Ok(Self {
            product_name,
            product_slug,
            home_dir_name,
            home_marker_file,
        })
    }

    pub fn product_name(&self) -> &str {
        &self.product_name
    }

    pub fn product_slug(&self) -> &str {
        &self.product_slug
    }

    pub fn home_dir_name(&self) -> &str {
        &self.home_dir_name
    }

    pub fn home_marker_file(&self) -> &str {
        &self.home_marker_file
    }

    /// Resolve this product's home under the current user's home directory.
    pub fn resolve_home(&self) -> std::io::Result<AbsolutePathBuf> {
        let user_home = home_dir().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not find home directory",
            )
        })?;
        self.resolve_home_from(user_home)
    }

    /// Resolve this product's home under an explicitly supplied user home.
    pub fn resolve_home_from<P: AsRef<Path>>(
        &self,
        user_home: P,
    ) -> std::io::Result<AbsolutePathBuf> {
        let user_home = AbsolutePathBuf::from_absolute_path_checked(user_home)?;
        Ok(user_home.join(&self.home_dir_name))
    }
}

fn validate_identity_component(kind: &str, value: &str) -> std::io::Result<()> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
    {
        return Err(invalid_identity_component(kind, value));
    }
    Ok(())
}

fn invalid_identity_component(kind: &str, value: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("{kind} must be a non-empty single path component, got {value:?}"),
    )
}

/// Returns the path to the Ruxex configuration directory, which can be
/// specified by the `RUXEX_HOME` environment variable. If not set, defaults to
/// `~/{DEFAULT_RUXEX_HOME_DIR_NAME}`.
///
/// - If `RUXEX_HOME` is set, the value must exist and be a directory. The
///   value will be canonicalized and this function will Err otherwise.
/// - If `RUXEX_HOME` is not set, this function does not verify that the
///   directory exists.
pub fn find_ruxex_home() -> std::io::Result<AbsolutePathBuf> {
    let ruxex_home_env = std::env::var("RUXEX_HOME")
        .ok()
        .filter(|val| !val.is_empty());
    find_ruxex_home_from_env(ruxex_home_env.as_deref())
}

/// Resolve a host product home while preserving the explicit legacy override.
pub fn find_product_home(identity: &ProductIdentity) -> std::io::Result<AbsolutePathBuf> {
    if std::env::var_os("RUXEX_HOME").is_some_and(|value| !value.is_empty()) {
        find_ruxex_home()
    } else {
        identity.resolve_home()
    }
}

fn find_ruxex_home_from_env(ruxex_home_env: Option<&str>) -> std::io::Result<AbsolutePathBuf> {
    // Honor the `RUXEX_HOME` environment variable when it is set to allow users
    // (and tests) to override the default location.
    match ruxex_home_env {
        Some(val) => {
            let path = PathBuf::from(val);
            let metadata = std::fs::metadata(&path).map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("RUXEX_HOME points to {val:?}, but that path does not exist"),
                ),
                _ => std::io::Error::new(
                    err.kind(),
                    format!("failed to read RUXEX_HOME {val:?}: {err}"),
                ),
            })?;

            if !metadata.is_dir() {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("RUXEX_HOME points to {val:?}, but that path is not a directory"),
                ))
            } else {
                let canonical = path.canonicalize().map_err(|err| {
                    std::io::Error::new(
                        err.kind(),
                        format!("failed to canonicalize RUXEX_HOME {val:?}: {err}"),
                    )
                })?;
                AbsolutePathBuf::from_absolute_path(canonical)
            }
        }
        None => {
            let mut p = home_dir().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not find home directory",
                )
            })?;
            p.push(DEFAULT_RUXEX_HOME_DIR_NAME);
            AbsolutePathBuf::from_absolute_path(p)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ProductIdentity, find_ruxex_home_from_env};
    use dirs::home_dir;
    use pretty_assertions::assert_eq;
    use ruxex_utils_absolute_path::AbsolutePathBuf;
    use std::fs;
    use std::io::ErrorKind;
    use tempfile::TempDir;

    #[test]
    fn find_ruxex_home_env_missing_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let missing = temp_home.path().join("missing-ruxex-home");
        let missing_str = missing
            .to_str()
            .expect("missing ruxex home path should be valid utf-8");

        let err = find_ruxex_home_from_env(Some(missing_str)).expect_err("missing RUXEX_HOME");
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert!(
            err.to_string().contains("RUXEX_HOME"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_ruxex_home_env_file_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let file_path = temp_home.path().join("ruxex-home.txt");
        fs::write(&file_path, "not a directory").expect("write temp file");
        let file_str = file_path
            .to_str()
            .expect("file ruxex home path should be valid utf-8");

        let err = find_ruxex_home_from_env(Some(file_str)).expect_err("file RUXEX_HOME");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("not a directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_ruxex_home_env_valid_directory_canonicalizes() {
        let temp_home = TempDir::new().expect("temp home");
        let temp_str = temp_home
            .path()
            .to_str()
            .expect("temp ruxex home path should be valid utf-8");

        let resolved = find_ruxex_home_from_env(Some(temp_str)).expect("valid RUXEX_HOME");
        let expected = temp_home
            .path()
            .canonicalize()
            .expect("canonicalize temp home");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn find_ruxex_home_without_env_uses_default_home_dir() {
        let resolved =
            find_ruxex_home_from_env(/*ruxex_home_env*/ None).expect("default RUXEX_HOME");
        let mut expected = home_dir().expect("home dir");
        expected.push(super::DEFAULT_RUXEX_HOME_DIR_NAME);
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn ruxex_product_identity_has_stable_defaults() {
        let identity = ProductIdentity::ruxex();

        assert_eq!(identity.product_name(), "Ruxex");
        assert_eq!(identity.product_slug(), "ruxex");
        assert_eq!(identity.home_dir_name(), ".ruxex");
        assert_eq!(identity.home_marker_file(), "ruxex-home.json");
    }

    #[test]
    fn custom_product_identity_resolves_home_under_supplied_user_home() {
        let identity = ProductIdentity::new(
            "Example Agent",
            "example-agent",
            ".example-agent",
            "example-agent-home.json",
        )
        .expect("valid product identity");
        let user_home = TempDir::new().expect("temp user home");

        let resolved = identity
            .resolve_home_from(user_home.path())
            .expect("custom product home");
        let expected = AbsolutePathBuf::from_absolute_path(user_home.path().join(".example-agent"))
            .expect("absolute custom home");

        assert_eq!(resolved, expected);
    }

    #[test]
    fn product_identity_rejects_unsafe_path_components() {
        for value in ["", ".", "..", "nested/name", r"nested\name"] {
            let err = ProductIdentity::new("Example Agent", value, ".example-agent", "marker.json")
                .expect_err("unsafe product slug should be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput, "value: {value:?}");
        }

        for value in ["", ".", "..", "nested/name", r"nested\name"] {
            let err = ProductIdentity::new("Example Agent", "example-agent", value, "marker.json")
                .expect_err("unsafe home directory should be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput, "value: {value:?}");
        }

        for value in ["", ".", "..", "nested/name", r"nested\name"] {
            let err =
                ProductIdentity::new("Example Agent", "example-agent", ".example-agent", value)
                    .expect_err("unsafe marker file should be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput, "value: {value:?}");
        }
    }
}
