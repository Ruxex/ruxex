//! Safe resolution and materialization metadata for marketplace-declared plugin assets.

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

/// The normalized source kind that controls how marketplace assets are obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketplaceAssetSourceKind {
    Local,
    Git,
    Npm,
}

/// Validated local asset locations ready for the app-server protocol.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarketplaceAssetFiles {
    pub composer_icon: Option<AbsolutePathBuf>,
    pub logo: Option<AbsolutePathBuf>,
    pub logo_dark: Option<AbsolutePathBuf>,
    pub screenshots: Vec<AbsolutePathBuf>,
}

/// Public URLs that a client may resolve and validate before rendering.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarketplaceAssetUrls {
    pub composer_icon_url: Option<String>,
    pub logo_url: Option<String>,
    pub logo_url_dark: Option<String>,
    pub screenshot_urls: Vec<String>,
}

/// Classifies an already-normalized source without inspecting its URL syntax.
pub fn classify_marketplace_asset_source(
    source: &MarketplacePluginSource,
) -> MarketplaceAssetSourceKind {
    match source {
        MarketplacePluginSource::Local { .. } => MarketplaceAssetSourceKind::Local,
        MarketplacePluginSource::Git { .. } => MarketplaceAssetSourceKind::Git,
        MarketplacePluginSource::Npm { .. } => MarketplaceAssetSourceKind::Npm,
    }
}

/// Resolves existing local interface assets while enforcing both repository roots.
///
/// Missing or unsafe optional assets are omitted. A plugin root outside the marketplace root
/// invalidates the whole local asset set.
pub fn resolve_local_marketplace_asset_files(
    marketplace_root: &Path,
    plugin_root: &Path,
    interface: &crate::manifest::PluginManifestInterface,
) -> Option<MarketplaceAssetFiles> {
    let marketplace_root = marketplace_root.canonicalize().ok()?;
    let plugin_root = plugin_root.canonicalize().ok()?;
    if !plugin_root.starts_with(&marketplace_root) {
        return None;
    }

    let resolve = |path: &AbsolutePathBuf| {
        let path = path.as_path().canonicalize().ok()?;
        if !path.is_file()
            || !path.starts_with(&plugin_root)
            || !path.starts_with(&marketplace_root)
        {
            return None;
        }
        AbsolutePathBuf::try_from(path).ok()
    };

    Some(MarketplaceAssetFiles {
        composer_icon: interface.composer_icon.as_ref().and_then(resolve),
        logo: interface.logo.as_ref().and_then(resolve),
        logo_dark: interface.logo_dark.as_ref().and_then(resolve),
        screenshots: interface.screenshots.iter().filter_map(resolve).collect(),
    })
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
    use crate::manifest::PluginManifestInterface;
    use crate::marketplace::MarketplacePluginSource;
    use ruxex_http_client::OutboundProxyPolicy;
    use ruxex_utils_absolute_path::AbsolutePathBuf;
    use std::fs;
    use tempfile::tempdir;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;
    use wiremock::matchers::method;
    use wiremock::matchers::path;

    #[test]
    fn classifies_asset_source_from_the_normalized_source_kind() {
        assert_eq!(
            classify_marketplace_asset_source(&MarketplacePluginSource::Local {
                path: AbsolutePathBuf::try_from("/tmp/plugin").unwrap(),
            }),
            MarketplaceAssetSourceKind::Local
        );
        assert_eq!(
            classify_marketplace_asset_source(&MarketplacePluginSource::Git {
                url: "https://example.com/not-a-git-looking-url".to_string(),
                path: None,
                ref_name: None,
                sha: None,
            }),
            MarketplaceAssetSourceKind::Git
        );
        assert_eq!(
            classify_marketplace_asset_source(&MarketplacePluginSource::Npm {
                package: "weather".to_string(),
                version: None,
                registry: None,
            }),
            MarketplaceAssetSourceKind::Npm
        );
    }

    #[test]
    fn resolves_only_existing_assets_inside_the_plugin_and_marketplace_roots() {
        let root = tempdir().unwrap();
        let plugin_root = root.path().join("plugins/weather");
        let assets_root = plugin_root.join("assets");
        fs::create_dir_all(&assets_root).unwrap();
        fs::write(assets_root.join("logo.svg"), b"<svg />").unwrap();
        fs::write(root.path().join("outside.svg"), b"<svg />").unwrap();

        let interface = PluginManifestInterface {
            logo: Some(AbsolutePathBuf::try_from(assets_root.join("logo.svg")).unwrap()),
            logo_dark: Some(AbsolutePathBuf::try_from(root.path().join("outside.svg")).unwrap()),
            ..Default::default()
        };

        let files = resolve_local_marketplace_asset_files(root.path(), &plugin_root, &interface)
            .expect("plugin root should be inside marketplace root");

        assert_eq!(
            files.logo.as_ref().map(AbsolutePathBuf::as_path),
            Some(assets_root.join("logo.svg").as_path())
        );
        assert!(files.logo_dark.is_none());
    }

    #[test]
    fn rejects_a_plugin_root_outside_the_marketplace_root() {
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let interface = PluginManifestInterface::default();

        assert!(
            resolve_local_marketplace_asset_files(root.path(), outside.path(), &interface)
                .is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_asset_symlinks_that_escape_the_plugin_root() {
        use std::os::unix::fs::symlink;

        let root = tempdir().unwrap();
        let plugin_root = root.path().join("plugins/weather");
        fs::create_dir_all(&plugin_root).unwrap();
        let outside = root.path().join("secret.svg");
        fs::write(&outside, b"<svg />").unwrap();
        symlink(&outside, plugin_root.join("logo.svg")).unwrap();
        let interface = PluginManifestInterface {
            logo: Some(AbsolutePathBuf::try_from(plugin_root.join("logo.svg")).unwrap()),
            ..Default::default()
        };

        let files =
            resolve_local_marketplace_asset_files(root.path(), &plugin_root, &interface).unwrap();
        assert!(files.logo.is_none());
    }

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
