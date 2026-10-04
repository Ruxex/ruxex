use super::runtime_paths;
use pretty_assertions::assert_eq;
use std::path::PathBuf;

#[test]
fn primary_runtime_path_does_not_depend_on_local_app_data() {
    let user_profile = PathBuf::from(r"C:\Users\user");

    assert_eq!(
        runtime_paths(Some(user_profile)),
        vec![PathBuf::from(r"C:\Users\user\.cache\genex-runtimes")]
    );
}
