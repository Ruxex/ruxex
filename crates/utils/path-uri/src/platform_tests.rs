//! Coverage for executor platform metadata normalization.

use super::Platform;

#[test]
fn platform_metadata_preserves_unknown_values() {
    for (metadata, expected) in [
        (Some("linux"), Platform::Linux),
        (Some("macos"), Platform::Macos),
        (Some("windows"), Platform::Windows),
        (None, Platform::Unknown),
        (Some("freebsd"), Platform::Unknown),
        (Some("Windows"), Platform::Unknown),
        (Some(""), Platform::Unknown),
    ] {
        assert_eq!(
            Platform::from_platform_os(metadata),
            expected,
            "{metadata:?}"
        );
    }
}

#[test]
fn native_platform_matches_process_metadata() {
    assert_eq!(
        Platform::native(),
        Platform::from_platform_os(Some(std::env::consts::OS)),
    );
}

#[test]
fn path_convention_is_known_only_for_supported_posix_platforms() {
    assert_eq!(
        Platform::Linux.path_convention(),
        Some(crate::PathConvention::Posix)
    );
    assert_eq!(
        Platform::Macos.path_convention(),
        Some(crate::PathConvention::Posix)
    );
    assert_eq!(Platform::Windows.path_convention(), None);
    assert_eq!(Platform::Unknown.path_convention(), None);
}
