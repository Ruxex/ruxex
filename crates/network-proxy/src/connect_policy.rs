use crate::policy::is_non_public_ip;
use crate::runtime::HostBlockDecision;
use crate::state::NetworkProxyState;
#[cfg(target_os = "macos")]
use crate::system_dns::SystemDnsResolver;
use rama_core::Service;
use rama_core::error::BoxError;
#[cfg(not(target_os = "macos"))]
use rama_core::error::BoxErrorExt as _;
use rama_core::error::ErrorExt as _;
use rama_core::error::extra::OpaqueError;
#[cfg(not(target_os = "macos"))]
use rama_core::extensions::ExtensionsRef;
#[cfg(target_os = "macos")]
use rama_dns::client::DnsConnector;
use rama_net::ConnectorTargetInputExt;
use rama_net::TransportProtocolInputExt;
use rama_net::address::Host;
use rama_net::address::HostWithPort;
use rama_net::address::ProxyAddress;
use rama_net::client::ConnectorTarget;
use rama_net::client::EstablishedClientConnection;
#[cfg(not(target_os = "macos"))]
use rama_net::stream::Socket;
#[cfg(not(target_os = "macos"))]
use rama_net::stream::SocketInfo;
#[cfg(not(target_os = "macos"))]
use rama_net::transport::TransportProtocol;
use rama_tcp::TcpStream;
use rama_tcp::client::TcpStreamConnector;
#[cfg(target_os = "macos")]
use rama_tcp::client::service::TcpConnector;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct TargetCheckedTcpConnector {
    state: Arc<NetworkProxyState>,
}

impl TargetCheckedTcpConnector {
    pub(crate) fn new(state: Arc<NetworkProxyState>) -> Self {
        Self { state }
    }
}

impl<Input> Service<Input> for TargetCheckedTcpConnector
where
    Input: ConnectorTargetInputExt + TransportProtocolInputExt + Send + 'static,
{
    type Output = EstablishedClientConnection<TcpStream, Input>;
    type Error = BoxError;

    async fn serve(&self, input: Input) -> Result<Self::Output, Self::Error> {
        #[cfg(not(target_os = "macos"))]
        {
            if let Some(proxy) = input.extensions().self_get_ref::<ProxyAddress>() {
                input
                    .extensions()
                    .insert(ConnectorTarget(proxy.address.clone()));
                return serve_resolved_target(input, ()).await;
            }

            let target = connector_target_without_parent(&input)
                .ok_or_else(|| OpaqueError::from_static_str("network target is missing a port"))?;
            return serve_resolved_target(
                input,
                TargetCheckedStreamConnector {
                    state: self.state.clone(),
                    target,
                },
            )
            .await;
        }

        #[cfg(target_os = "macos")]
        {
            let connector = TcpConnector::new();
            if let Some(proxy) = input.extensions().self_get_ref::<ProxyAddress>() {
                input
                    .extensions()
                    .insert(ConnectorTarget(proxy.address.clone()));
                return DnsConnector::with_resolver(connector, SystemDnsResolver::default())
                    .serve(input)
                    .await;
            }

            let target = connector_target_without_parent(&input)
                .ok_or_else(|| OpaqueError::from_static_str("network target is missing a port"))?;
            input.extensions().insert(ConnectorTarget(target.clone()));
            let connector = connector.with_connector(TargetCheckedStreamConnector {
                state: self.state.clone(),
                target,
            });
            return DnsConnector::with_resolver(connector, SystemDnsResolver::default())
                .serve(input)
                .await;
        }
    }
}

#[cfg(not(target_os = "macos"))]
async fn serve_resolved_target<Input, Connector>(
    input: Input,
    connector: Connector,
) -> Result<EstablishedClientConnection<TcpStream, Input>, BoxError>
where
    Input: ConnectorTargetInputExt + TransportProtocolInputExt + Send + 'static,
    Connector: TcpStreamConnector<Error: Into<BoxError>> + Send + 'static,
{
    if input.transport_protocol() == Some(TransportProtocol::Udp) {
        return Err(BoxError::from_static_str(
            "Tcp Connector Service cannot establish a UDP transport",
        ));
    }

    let target = connector_target_without_parent(&input)
        .ok_or_else(|| OpaqueError::from_static_str("network target is missing a port"))?;
    let addresses = if let Ok(ip) = target.host.try_as_ip() {
        vec![SocketAddr::from((ip, target.port))]
    } else {
        tokio::net::lookup_host((target.host.to_string(), target.port))
            .await
            .map_err(Into::<BoxError>::into)?
            .collect::<Vec<_>>()
    };
    let mut last_error = None;
    for addr in addresses {
        match connector.connect(addr).await.map_err(Into::into) {
            Ok(conn) => {
                let socket_info = SocketInfo::new(conn.local_addr().ok(), addr.into());
                conn.extensions().insert(socket_info);
                return Ok(EstablishedClientConnection { input, conn });
            }
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error
        .unwrap_or_else(|| OpaqueError::from_static_str("DNS lookup returned no addresses").into()))
}

fn connector_target_without_parent<Input>(input: &Input) -> Option<HostWithPort>
where
    Input: ConnectorTargetInputExt,
{
    input
        .extensions()
        .self_get_ref::<ConnectorTarget>()
        .map(|target| target.0.clone())
        .or_else(|| {
            input
                .authority()
                .and_then(|authority| authority.into_host_with_port(input.protocol_default_port()))
        })
}

#[derive(Clone)]
struct TargetCheckedStreamConnector {
    state: Arc<NetworkProxyState>,
    target: HostWithPort,
}

impl TcpStreamConnector for TargetCheckedStreamConnector {
    type Error = BoxError;

    async fn connect(&self, addr: SocketAddr) -> Result<TcpStream, Self::Error> {
        if is_non_public_ip(addr.ip()) && !self.allows_non_public_target(addr).await? {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "network target rejected by policy",
            )
            .into());
        }

        tokio::net::TcpStream::connect(addr)
            .await
            .map(TcpStream::from)
            .map_err(Into::into)
    }
}

impl TargetCheckedStreamConnector {
    async fn allows_non_public_target(&self, addr: SocketAddr) -> Result<bool, BoxError> {
        if self.state.allow_local_binding().await.map_err(|err| {
            let err: BoxError = err.into();
            err.into_opaque_error()
                .context("read network proxy config")
                .into_box_error()
        })? {
            return Ok(true);
        }

        if !target_matches_non_public_addr(&self.target.host, addr.ip()) {
            return Ok(false);
        }

        self.state
            .host_blocked(&self.target.host.to_string(), self.target.port)
            .await
            .map(|decision| decision == HostBlockDecision::Allowed)
            .map_err(|err| {
                let err: BoxError = err.into();
                err.into_opaque_error()
                    .context("evaluate network proxy target")
                    .into_box_error()
            })
    }
}

pub(crate) fn is_non_public_target(host: &Host) -> bool {
    match host {
        Host::Address(ip) => is_non_public_ip(*ip),
        Host::Name(name) => name
            .as_str()
            .trim_end_matches('.')
            .eq_ignore_ascii_case("localhost"),
        _ => false,
    }
}

fn target_matches_non_public_addr(host: &Host, addr: std::net::IpAddr) -> bool {
    match host {
        Host::Address(ip) => *ip == addr,
        Host::Name(name) => {
            name.as_str()
                .trim_end_matches('.')
                .eq_ignore_ascii_case("localhost")
                && addr.is_loopback()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NetworkProxyConfig;
    use crate::state::network_proxy_state_for_policy;
    use rama_core::extensions::ExtensionsRef;
    use rama_net::address::HostWithPort;
    use rama_net::stream::Socket;
    use std::net::Ipv4Addr;
    use tokio::net::TcpListener;

    #[tokio::test(flavor = "current_thread")]
    async fn direct_connector_rejects_non_public_target_when_local_binding_disabled() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind local listener");
        let target = listener.local_addr().expect("local addr");
        let connector = TargetCheckedTcpConnector::new(Arc::new(network_proxy_state_for_policy(
            NetworkProxyConfig::default(),
        )));

        let request: rama_net::client::Request =
            rama_net::client::Request::new(HostWithPort::from(target));
        let err = Service::serve(&connector, request)
            .await
            .expect_err("local target should be rejected");

        assert!(
            format!("{err:?}").contains("network target rejected by policy"),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn direct_connector_allows_non_public_target_when_local_binding_enabled() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind local listener");
        let target = listener.local_addr().expect("local addr");
        let connector = TargetCheckedTcpConnector::new(Arc::new(network_proxy_state_for_policy(
            NetworkProxyConfig {
                allow_local_binding: true,
                ..NetworkProxyConfig::default()
            },
        )));

        let request: rama_net::client::Request =
            rama_net::client::Request::new(HostWithPort::from(target));
        let result = Service::serve(&connector, request).await;

        assert!(result.is_ok(), "local target should be allowed: {result:?}");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn direct_connector_allows_explicitly_allowlisted_non_public_target() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind local listener");
        let target = listener.local_addr().expect("local addr");
        let mut config = NetworkProxyConfig::default();
        config.set_allowed_domains(vec![target.ip().to_string()]);
        let connector =
            TargetCheckedTcpConnector::new(Arc::new(network_proxy_state_for_policy(config)));

        let request: rama_net::client::Request =
            rama_net::client::Request::new(HostWithPort::from(target));
        let result = Service::serve(&connector, request).await;

        assert!(
            result.is_ok(),
            "explicitly allowlisted local target should be allowed: {result:?}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn direct_connector_allows_explicitly_allowlisted_localhost_target() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind local listener");
        let target = listener.local_addr().expect("local addr");
        let mut config = NetworkProxyConfig::default();
        config.set_allowed_domains(vec!["localhost".to_string()]);
        let connector =
            TargetCheckedTcpConnector::new(Arc::new(network_proxy_state_for_policy(config)));

        let request: rama_net::client::Request =
            rama_net::client::Request::new(HostWithPort::new(Host::LOCALHOST_NAME, target.port()));
        let result = Service::serve(&connector, request).await;

        assert!(
            result.is_ok(),
            "explicitly allowlisted localhost target should be allowed: {result:?}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn proxy_connector_uses_proxy_address_as_target() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind local listener");
        let target = listener.local_addr().expect("local addr");
        let connector = TargetCheckedTcpConnector::new(Arc::new(network_proxy_state_for_policy(
            NetworkProxyConfig::default(),
        )));
        let request = rama_net::client::Request::new(
            HostWithPort::try_from("destination.invalid:80").expect("valid destination"),
        );
        request.extensions().insert(
            ProxyAddress::try_from(format!("http://{}", target)).expect("valid upstream proxy"),
        );

        let connection = Service::serve(&connector, request)
            .await
            .expect("connect to proxy");

        assert_eq!(connection.conn.peer_addr().expect("peer addr"), target);
    }

    #[test]
    fn resolved_private_address_does_not_match_allowlisted_hostname() {
        let host = Host::Name("example.com".parse().expect("valid domain"));

        assert!(!target_matches_non_public_addr(
            &host,
            Ipv4Addr::LOCALHOST.into()
        ));
    }
}
