use super::shell_approval_command;
use crate::shell::Shell;
use crate::shell::ShellType;
use pretty_assertions::assert_eq;
use std::path::PathBuf;

#[test]
fn parent_directory_traversal_is_not_a_trusted_system_shell() {
    let configured_executable = PathBuf::from("/bin/sh");
    let unfamiliar_executable = PathBuf::from("/bin/../workspace/bash");
    let shell = Shell {
        shell_type: ShellType::Sh,
        shell_path: configured_executable,
    };
    let command = vec![
        unfamiliar_executable.to_string_lossy().into_owned(),
        "-c".to_string(),
        "ls".to_string(),
    ];

    assert_eq!(
        shell_approval_command(&command, &shell),
        &command[..1],
    );
}
