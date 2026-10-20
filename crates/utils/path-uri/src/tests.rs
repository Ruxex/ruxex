use super::*;

#[test]
fn parses_and_joins_posix_paths() {
    let path = PathUri::parse("file:///tmp/project").unwrap();
    assert_eq!(
        path.join("src/main.rs").unwrap().to_string(),
        "file:///tmp/project/src/main.rs"
    );
    assert_eq!(
        path.join_descendant("src/main.rs").unwrap().to_string(),
        "file:///tmp/project/src/main.rs"
    );
}

#[test]
fn parent_stops_at_posix_root() {
    let root = PathUri::parse("file:///tmp").unwrap();
    assert_eq!(root.parent().unwrap().to_string(), "file:///");
    assert!(PathUri::parse("file:///").unwrap().parent().is_none());
}

#[test]
fn relative_path_requires_containment() {
    let base = PathUri::parse("file:///tmp/project").unwrap();
    let child = PathUri::parse("file:///tmp/project/src").unwrap();
    assert_eq!(child.relative_path_from(&base).as_deref(), Some("src"));
    assert!(base.relative_path_from(&child).is_none());
}
