//! This macOS-only module implements DNS resolution
//! using macOS's native system resolver. We have to
//! do this rather than relying on the default behavior
//! (which reads /etc/resolv.conf) since default behavior
//! breaks Ruxex's ability to interact with network paths
//! that are provided by applications that modify / hook
//! into macOS DNS behavior (i.e. proxies, VPNs, etc)
use rama_core::error::BoxError;
use rama_core::futures::{Stream, TryStreamExt as _};
use rama_dns::client::AppleDnsResolver;
use rama_dns::client::resolver::DnsAddressResolver;
use rama_net::address::Domain;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

/// Resolve TCP destinations through macOS's native resolver, including supplemental
/// resolvers and VPN split DNS that are not represented in `/etc/resolv.conf`.
#[derive(Clone)]
pub(crate) struct SystemDnsResolver(AppleDnsResolver);

impl Default for SystemDnsResolver {
    fn default() -> Self {
        Self(AppleDnsResolver::new())
    }
}

impl DnsAddressResolver for SystemDnsResolver {
    type Error = BoxError;

    fn lookup_ipv4(
        &self,
        domain: Domain,
    ) -> impl Stream<Item = Result<Ipv4Addr, Self::Error>> + Send + '_ {
        self.0.lookup_ipv4(domain)
    }

    fn lookup_ipv6(
        &self,
        domain: Domain,
    ) -> impl Stream<Item = Result<Ipv6Addr, Self::Error>> + Send + '_ {
        self.0.lookup_ipv6(domain)
    }
}

impl SystemDnsResolver {
    /// Compatibility helpers retained for the resolver behavior tests.
    pub(crate) async fn ipv4_lookup(&self, domain: Domain) -> Result<Vec<Ipv4Addr>, BoxError> {
        self.lookup_ipv4(domain).try_collect().await
    }

    pub(crate) async fn ipv6_lookup(&self, domain: Domain) -> Result<Vec<Ipv6Addr>, BoxError> {
        self.lookup_ipv6(domain).try_collect().await
    }
}

#[cfg(test)]
#[path = "system_dns_tests.rs"]
mod tests;
