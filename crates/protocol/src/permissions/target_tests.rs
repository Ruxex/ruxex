use super::*;
use crate::permissions::FileSystemAccessMode;
use crate::permissions::FileSystemSandboxEntry;

#[test]
fn target_policy_resolves_posix_paths() {
    let cwd = PathUri::parse("file:///work/repo").unwrap();
    let policy = FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry::new(
        FileSystemPath::Path {
            path: cwd.join("private").unwrap(),
        },
        FileSystemAccessMode::Deny,
    )]);
    let context = FileSystemSandboxPolicyContext {
        cwd: &cwd,
        workspace_roots: std::slice::from_ref(&cwd),
        user_home_dir: Some(&cwd),
        temporary_directories: Some(&[]),
    };
    let matcher = ReadDenyMatcher::try_new_with_context(&policy, &context)
        .unwrap()
        .unwrap();
    assert!(matcher.is_read_denied_uri(&cwd.join("private/file").unwrap(), &context));
}
