use super::*;
use crate::PathConvention;

#[test]
fn resolves_and_normalizes_posix_config_path() {
    let base = PathUri::parse("file:///tmp/project").unwrap();
    let resolved =
        PathUri::resolve_config_path_uri("src/", PathConvention::Posix, Some(&base), None).unwrap();
    assert_eq!(resolved.to_string(), "file:///tmp/project/src");
}

#[test]
fn rejects_null_config_path() {
    assert!(PathUri::validate_config_path_text("/tmp/\0", PathConvention::Posix).is_err());
}
