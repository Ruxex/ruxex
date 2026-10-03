#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    genex_windows_sandbox::setup_helper_main()
}

#[cfg(not(target_os = "windows"))]
fn main() {
    panic!("genex-windows-sandbox-setup is Windows-only");
}
