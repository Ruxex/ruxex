use super::*;
use crate::PathConvention;

#[test]
fn renders_posix_uri() {
    let uri = PathUri::parse("file:///tmp/project").unwrap();
    assert_eq!(
        LegacyAppPathString::from_path_uri(&uri, PathConvention::Posix)
            .unwrap()
            .as_str(),
        "/tmp/project"
    );
}

#[test]
fn resolves_relative_path_against_cwd() {
    let cwd = PathUri::parse("file:///tmp/project").unwrap();
    let path = LegacyAppPathString::from_string("src/main.rs");
    assert_eq!(
        path.resolve_against(&cwd, None).unwrap().to_string(),
        "file:///tmp/project/src/main.rs"
    );
}
