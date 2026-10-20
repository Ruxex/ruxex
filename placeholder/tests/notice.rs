#[test]
fn notice_explains_that_the_git_dependency_is_required() {
    let message = ruxex::notice();

    assert!(message.contains("Git dependency"));
    assert!(message.contains("github.com/Ruxex/ruxex"));
}
