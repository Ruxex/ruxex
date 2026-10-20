use super::is_safe_for_authorization;
use pretty_assertions::assert_eq;

#[test]
fn accepts_unambiguous_paths() {
    let paths = [
        "/provider/provider",
        "/provider/provider/issues/123",
        "/provider/provider/a..b",
        "/provider/provider/%20space",
        "/provider/provider/%E2%9C%93",
        "/provider/provider/contents/%2Egitignore",
        "/provider/provider/contents/.%2Egithub",
        "/provider/provider/%2e%2efoo",
        "/provider/provider/%2e%2e%2e",
    ];

    assert_eq!(
        paths.map(is_safe_for_authorization),
        [true, true, true, true, true, true, true, true, true]
    );
}

#[test]
fn rejects_paths_with_ambiguous_segments_or_encodings() {
    let paths = [
        "/provider/provider/../repo",
        "/provider/provider/./issues",
        "/provider/provider\\..\\repo",
        "/provider/provider/%2e%2e/repo",
        "/provider/provider/%2E%2E/repo",
        "/provider/provider/%2f..%2frepo",
        "/provider/provider/%5c..%5crepo",
        "/provider/provider/%252e%252e/repo",
        "/provider/provider/%",
        "/provider/provider/%2",
        "/provider/provider/%zz",
    ];

    assert_eq!(
        paths.map(is_safe_for_authorization),
        [
            false, false, false, false, false, false, false, false, false, false, false
        ]
    );
}
