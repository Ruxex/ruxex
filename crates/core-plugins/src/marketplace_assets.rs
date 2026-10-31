//! Safe resolution of marketplace-declared plugin assets to public GitHub URLs.
//!
//! This module deliberately resolves URLs only. It does not fetch image bytes; clients must fetch,
//! validate, sanitize, and cache the returned URLs before rendering them.

use crate::ConfiguredMarketplace;
use crate::ConfiguredMarketplacePlugin;
use crate::marketplace::MarketplacePluginSource;
use futures::StreamExt;
use ruxex_http_client::ClientRouteClass;
use ruxex_http_client::HttpClientBuilder;
use ruxex_http_client::HttpClientFactory;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use url::Url;

const RAW_GITHUB_HOST: &str = "raw.githubusercontent.com";
const REMOTE_MANIFEST_MAX_BYTES: usize = 64 * 1024;
const REMOTE_MANIFEST_TIMEOUT: Duration = Duration::from_secs(3);
const REMOTE_MANIFEST_CANDIDATES: &[&str] = &[
    ".codex-plugin/plugin.json",
    "plugin.json",
    ".ruxex-plugin/plugin.json",
    ".claude-plugin/plugin.json",
    ".cursor-plugin/plugin.json",
];

/// Where a configured marketplace came from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MarketplaceOrigin {
    /// A public GitHub repository used as the marketplace source.
    Git {
        url: String,
        ref_name: Option<String>,
        revision: Option<String>,
    },
    /// A marketplace supplied directly from the local filesystem.
    Local { path: AbsolutePathBuf },
    /// A marketplace that cannot be associated with a safe public origin.
    #[default]
    Unknown,
}

/// Asset paths as declared by a plugin manifest.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarketplaceAssetPaths {
    pub composer_icon: Option<String>,
    pub logo: Option<String>,
    pub logo_dark: Option<String>,
    pub screenshots: Vec<String>,
}

/// Public URLs that a client may resolve and validate before rendering.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarketplaceAssetUrls {
    pub composer_icon_url: Option<String>,
    pub logo_url: Option<String>,
    pub logo_url_dark: Option<String>,
    pub screenshot_urls: Vec<String>,
}

/// Resolves marketplace asset metadata while leaving asset-byte fetching to clients.
#[derive(Clone)]
pub struct MarketplaceAssetResolver {
    http_client_factory: HttpClientFactory,
}

impl MarketplaceAssetResolver {
    pub fn new(http_client_factory: HttpClientFactory) -> Self {
        Self {
            http_client_factory,
        }
    }

    /// Resolves assets for all plugins in one discovered marketplace.
    pub async fn resolve_marketplace(
        &self,
        marketplace: &ConfiguredMarketplace,
    ) -> HashMap<String, MarketplaceAssetUrls> {
        let mut manifest_cache = HashMap::<String, Option<MarketplaceAssetPaths>>::new();
        let mut resolved = HashMap::new();
        for plugin in &marketplace.plugins {
            if let Some(urls) = self
                .resolve_plugin(marketplace, plugin, &mut manifest_cache)
                .await
            {
                resolved.insert(plugin.id.clone(), urls);
            }
        }
        resolved
    }

    async fn resolve_plugin(
        &self,
        marketplace: &ConfiguredMarketplace,
        plugin: &ConfiguredMarketplacePlugin,
        manifest_cache: &mut HashMap<String, Option<MarketplaceAssetPaths>>,
    ) -> Option<MarketplaceAssetUrls> {
        match &plugin.source {
            MarketplacePluginSource::Local { path } => {
                let paths = local_asset_paths(plugin.interface.as_ref()?, path.as_path())?;
                resolve_marketplace_asset_urls(
                    &marketplace.origin,
                    marketplace.root.as_path(),
                    &plugin.source,
                    &paths,
                )
            }
            MarketplacePluginSource::Git { .. } => {
                let mut paths = None;
                for candidate in REMOTE_MANIFEST_CANDIDATES {
                    let manifest_url = resolve_marketplace_asset_urls(
                        &marketplace.origin,
                        marketplace.root.as_path(),
                        &plugin.source,
                        &MarketplaceAssetPaths {
                            logo: Some((*candidate).to_string()),
                            ..Default::default()
                        },
                    )?
                    .logo_url?;
                    let fetched = if let Some(cached) = manifest_cache.get(&manifest_url) {
                        cached.clone()
                    } else {
                        let fetched = self.fetch_manifest_asset_paths(&manifest_url).await;
                        manifest_cache.insert(manifest_url, fetched.clone());
                        fetched
                    };
                    if fetched.is_some() {
                        paths = fetched;
                        break;
                    }
                }
                resolve_marketplace_asset_urls(
                    &marketplace.origin,
                    marketplace.root.as_path(),
                    &plugin.source,
                    &paths?,
                )
            }
            MarketplacePluginSource::Npm { .. } => None,
        }
    }

    async fn fetch_manifest_asset_paths(&self, url: &str) -> Option<MarketplaceAssetPaths> {
        let client = HttpClientBuilder::new()
            .without_redirects()
            .without_request_logging()
            .build_respecting_outbound_proxy_policy(
                &self.http_client_factory,
                url,
                ClientRouteClass::Other,
            )
            .ok()?;
        let response = client
            .get(url)
            .timeout(REMOTE_MANIFEST_TIMEOUT)
            .send()
            .await
            .ok()?;
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|length| length > REMOTE_MANIFEST_MAX_BYTES as u64)
        {
            return None;
        }

        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.ok()?;
            if body.len().saturating_add(chunk.len()) > REMOTE_MANIFEST_MAX_BYTES {
                return None;
            }
            body.extend_from_slice(&chunk);
        }
        parse_marketplace_manifest_asset_paths(&body)
    }
}

#[derive(Debug, Deserialize)]
struct RemotePluginManifest {
    #[serde(default)]
    interface: Option<RemotePluginInterface>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemotePluginInterface {
    composer_icon: Option<String>,
    logo: Option<String>,
    logo_dark: Option<String>,
    #[serde(default)]
    screenshots: Vec<String>,
}

/// Parses only UI asset paths from a remote plugin manifest.
pub fn parse_marketplace_manifest_asset_paths(body: &[u8]) -> Option<MarketplaceAssetPaths> {
    let manifest = serde_json::from_slice::<RemotePluginManifest>(body).ok()?;
    let interface = manifest.interface?;
    Some(MarketplaceAssetPaths {
        composer_icon: interface.composer_icon,
        logo: interface.logo,
        logo_dark: interface.logo_dark,
        screenshots: interface.screenshots,
    })
}

fn local_asset_paths(
    interface: &crate::manifest::PluginManifestInterface,
    plugin_root: &Path,
) -> Option<MarketplaceAssetPaths> {
    let relative =
        |path: &AbsolutePathBuf| relative_path(path.as_path().strip_prefix(plugin_root).ok()?);
    Some(MarketplaceAssetPaths {
        composer_icon: interface.composer_icon.as_ref().and_then(relative),
        logo: interface.logo.as_ref().and_then(relative),
        logo_dark: interface.logo_dark.as_ref().and_then(relative),
        screenshots: interface.screenshots.iter().filter_map(relative).collect(),
    })
}

/// Resolves manifest asset paths to raw GitHub URLs when the source and paths are safe.
///
/// `None` means that no safe public origin could be established. Individual invalid asset paths
/// are omitted from the result so one malformed optional asset does not hide valid siblings.
pub fn resolve_marketplace_asset_urls(
    origin: &MarketplaceOrigin,
    marketplace_root: &Path,
    source: &MarketplacePluginSource,
    paths: &MarketplaceAssetPaths,
) -> Option<MarketplaceAssetUrls> {
    let (repository_url, plugin_root, revision) = match source {
        MarketplacePluginSource::Git {
            url,
            path,
            ref_name,
            sha,
        } => {
            let repository_url = github_repository(url)?;
            let revision = sha.as_deref().or(ref_name.as_deref()).unwrap_or("HEAD");
            validate_reference(revision)?;
            (
                repository_url,
                path.clone().unwrap_or_default(),
                revision.to_string(),
            )
        }
        MarketplacePluginSource::Local { path } => {
            let MarketplaceOrigin::Git {
                url,
                ref_name,
                revision,
            } = origin
            else {
                return None;
            };
            let repository_url = github_repository(url)?;
            let relative_plugin_root = path.as_path().strip_prefix(marketplace_root).ok()?;
            let relative_plugin_root = relative_path(relative_plugin_root)?;
            let revision = revision
                .as_deref()
                .or(ref_name.as_deref())
                .unwrap_or("HEAD");
            validate_reference(revision)?;
            (repository_url, relative_plugin_root, revision.to_string())
        }
        MarketplacePluginSource::Npm { .. } => return None,
    };

    let mut base = raw_github_url(&repository_url, &revision)?;
    append_path_segments(&mut base, &plugin_root, true)?;

    Some(MarketplaceAssetUrls {
        composer_icon_url: append_asset_url(&base, paths.composer_icon.as_deref()),
        logo_url: append_asset_url(&base, paths.logo.as_deref()),
        logo_url_dark: append_asset_url(&base, paths.logo_dark.as_deref()),
        screenshot_urls: paths
            .screenshots
            .iter()
            .filter_map(|path| append_asset_url(&base, Some(path)))
            .collect(),
    })
}

fn append_asset_url(base: &Url, path: Option<&str>) -> Option<String> {
    let mut url = base.clone();
    append_path_segments(&mut url, path?, false)?;
    Some(url.to_string())
}

fn append_path_segments(url: &mut Url, path: &str, allow_empty: bool) -> Option<()> {
    let segments = if path.is_empty() && allow_empty {
        Vec::new()
    } else {
        safe_path_segments(path)?
    };
    for segment in segments {
        url.path_segments_mut().ok()?.push(segment);
    }
    Some(())
}

fn raw_github_url(repository: &str, revision: &str) -> Option<Url> {
    let mut url = Url::parse(&format!("https://{RAW_GITHUB_HOST}/")).ok()?;
    let repository = repository.strip_prefix("https://github.com/")?;
    for segment in repository.split('/') {
        url.path_segments_mut().ok()?.push(segment);
    }
    append_path_segments(&mut url, revision, false)?;
    Some(url)
}

fn github_repository(source: &str) -> Option<String> {
    let source = source.trim().trim_end_matches('/');
    let source = source.strip_suffix(".git").unwrap_or(source);
    let source = if source.contains("://") {
        let url = Url::parse(source).ok()?;
        (url.scheme() == "https").then_some(())?;
        (url.host_str() == Some("github.com")).then_some(())?;
        if url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some()
        {
            return None;
        }
        url.path().trim_matches('/').to_string()
    } else {
        source.to_string()
    };
    let mut segments = source.split('/');
    let owner = segments.next()?.trim();
    let repo = segments.next()?.trim();
    if segments.next().is_some()
        || owner.is_empty()
        || repo.is_empty()
        || !safe_reference_segment(owner)
        || !safe_reference_segment(repo)
    {
        return None;
    }
    Some(format!("https://github.com/{owner}/{repo}"))
}

fn relative_path(path: &Path) -> Option<String> {
    let path = path.to_str()?;
    let path = path.replace('\\', "/");
    let path = path.strip_prefix("./").unwrap_or(&path);
    let path = path.strip_prefix('/').unwrap_or(path);
    if path.is_empty() {
        Some(String::new())
    } else {
        safe_path_segments(path).map(|_| path.to_string())
    }
}

fn safe_path_segments(path: &str) -> Option<Vec<&str>> {
    if path.is_empty()
        || path.contains('\\')
        || path.contains('%')
        || path.contains(['?', '#'])
        || path.starts_with('/')
        || path.starts_with("//")
    {
        return None;
    }
    let segments = path.split('/').collect::<Vec<_>>();
    if segments
        .iter()
        .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
    {
        return None;
    }
    segments
        .iter()
        .all(|segment| !segment.chars().any(char::is_control))
        .then_some(segments)
}

fn validate_reference(reference: &str) -> Option<()> {
    let segments = safe_path_segments(reference)?;
    if reference == "@"
        || reference.contains("..")
        || reference.contains("@{")
        || reference.chars().any(|character| {
            character.is_whitespace() || matches!(character, '~' | '^' | ':' | '?' | '*' | '[')
        })
        || segments
            .iter()
            .any(|segment| segment.starts_with('.') || segment.ends_with('.'))
    {
        return None;
    }
    Some(())
}

fn safe_reference_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.contains(['/', '\\', '%', '?', '#', ':'])
        && !segment.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruxex_http_client::OutboundProxyPolicy;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;
    use wiremock::matchers::method;
    use wiremock::matchers::path;

    #[tokio::test]
    async fn fetches_bounded_manifest_metadata_without_following_redirects() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/manifest.json"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                br#"{"interface":{"logo":"assets/logo.svg"}}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let resolver = MarketplaceAssetResolver::new(HttpClientFactory::new(
            OutboundProxyPolicy::ReqwestDefault,
        ));

        let paths = resolver
            .fetch_manifest_asset_paths(&format!("{}/manifest.json", server.uri()))
            .await
            .expect("bounded manifest should be accepted");
        assert_eq!(paths.logo.as_deref(), Some("assets/logo.svg"));

        Mock::given(method("GET"))
            .and(path("/redirect.json"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", "/manifest.json"))
            .mount(&server)
            .await;
        assert!(
            resolver
                .fetch_manifest_asset_paths(&format!("{}/redirect.json", server.uri()))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn rejects_manifest_bodies_over_the_limit() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/large.json"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![
                b' ';
                REMOTE_MANIFEST_MAX_BYTES
                    + 1
            ]))
            .mount(&server)
            .await;
        let resolver = MarketplaceAssetResolver::new(HttpClientFactory::new(
            OutboundProxyPolicy::ReqwestDefault,
        ));

        assert!(
            resolver
                .fetch_manifest_asset_paths(&format!("{}/large.json", server.uri()))
                .await
                .is_none()
        );
    }
}
