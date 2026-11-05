#[cfg(feature = "cloudflare-provider")]
pub mod cloudflare;

#[cfg(feature = "cloudflare-provider")]
pub mod cloudflare_auth;

#[cfg(feature = "dummy-provider")]
pub mod dummy_provider;

#[cfg(feature = "dummy-provider")]
pub fn run_dummy_provider() -> anyhow::Result<()> {
    ruxex_arg0::arg0_dispatch_or_else(
        |arg0_paths| async move { dummy_provider::run(arg0_paths).await },
    )
}

#[cfg(all(test, feature = "cloudflare-provider"))]
mod tests {
    #[test]
    fn cloudflare_provider_factory_is_public() {
        let (_, descriptor) = super::cloudflare::provider_factory();

        assert_eq!(descriptor.provider_kind, "cloudflare");
    }
}
