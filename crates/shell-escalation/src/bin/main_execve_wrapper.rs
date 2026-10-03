#[cfg(not(unix))]
fn main() {
    eprintln!("genex-execve-wrapper is only implemented for UNIX");
    std::process::exit(1);
}

#[cfg(unix)]
pub use genex_shell_escalation::main_execve_wrapper as main;
