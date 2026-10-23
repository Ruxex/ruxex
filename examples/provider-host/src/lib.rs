#[cfg(feature = "dummy-provider")]
pub mod dummy_provider;

#[cfg(feature = "dummy-provider")]
pub fn run_dummy_provider() -> anyhow::Result<()> {
    ruxex_arg0::arg0_dispatch_or_else(
        |arg0_paths| async move { dummy_provider::run(arg0_paths).await },
    )
}
