//! Safe resolution and materialization metadata for marketplace-declared plugin assets.

use crate::marketplace::MarketplacePluginSource;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use std::fs;
use std::path::Path;

pub const MARKETPLACE_ASSET_MAX_BYTES: u64 = 8 * 1024 * 1024;

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

/// Validates a local marketplace asset before exposing its path to a client.
pub fn validate_marketplace_asset_file(
    path: &AbsolutePathBuf,
    allowed_root: &Path,
) -> Option<AbsolutePathBuf> {
    let allowed_root = allowed_root.canonicalize().ok()?;
    let requested = path.as_path();
    let metadata = fs::symlink_metadata(requested).ok()?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return None;
    }
    if metadata.len() > MARKETPLACE_ASSET_MAX_BYTES {
        return None;
    }
    let canonical = requested.canonicalize().ok()?;
    if !canonical.starts_with(&allowed_root) {
        return None;
    }
    let bytes = fs::read(&canonical).ok()?;
    if bytes.len() as u64 > MARKETPLACE_ASSET_MAX_BYTES {
        return None;
    }
    let mime = detect_marketplace_asset_mime(&bytes)?;
    if mime == "image/svg+xml" {
        validate_marketplace_svg(&bytes).ok()?
    }
    AbsolutePathBuf::try_from(canonical).ok()
}

/// Validates every local asset while preserving the optional-field omission behavior.
pub fn validate_marketplace_asset_files(
    files: &MarketplaceAssetFiles,
    allowed_root: &Path,
) -> MarketplaceAssetFiles {
    MarketplaceAssetFiles {
        composer_icon: files
            .composer_icon
            .as_ref()
            .and_then(|path| validate_marketplace_asset_file(path, allowed_root)),
        logo: files
            .logo
            .as_ref()
            .and_then(|path| validate_marketplace_asset_file(path, allowed_root)),
        logo_dark: files
            .logo_dark
            .as_ref()
            .and_then(|path| validate_marketplace_asset_file(path, allowed_root)),
        screenshots: files
            .screenshots
            .iter()
            .filter_map(|path| validate_marketplace_asset_file(path, allowed_root))
            .collect(),
    }
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
        let metadata = fs::symlink_metadata(path.as_path()).ok()?;
        if metadata.file_type().is_symlink() {
            return None;
        }
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

fn detect_marketplace_asset_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if bytes.len() >= 12
        && &bytes[4..8] == b"ftyp"
        && [b"avif".as_slice(), b"avis", b"mif1", b"msf1"]
            .iter()
            .any(|brand| &bytes[8..12] == *brand)
    {
        return Some("image/avif");
    }
    let text = std::str::from_utf8(bytes)
        .ok()?
        .trim_start_matches('\u{feff}')
        .trim_start();
    (text.starts_with("<svg") || (text.starts_with("<?xml") && text.contains("<svg")))
        .then_some("image/svg+xml")
}

fn validate_marketplace_svg(bytes: &[u8]) -> Result<(), ()> {
    let text = std::str::from_utf8(bytes).map_err(|_| ())?;
    let lower = text.to_ascii_lowercase();
    if !lower.contains("<svg")
        || [
            "<!doctype",
            "<!entity",
            "<script",
            "<foreignobject",
            "<iframe",
            "<object",
            "<embed",
            "<audio",
            "<video",
            "javascript:",
            "data:",
            "url(",
            "href=",
            "onload=",
            "onerror=",
            "onclick=",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return Err(());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PluginManifestInterface;
    use crate::marketplace::MarketplacePluginSource;
    use ruxex_utils_absolute_path::AbsolutePathBuf;
    use std::fs;
    use tempfile::tempdir;

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

    #[test]
    fn validates_image_signatures_and_rejects_unsafe_svg() {
        let root = tempdir().unwrap();
        let png = root.path().join("logo.png");
        let svg = root.path().join("logo.svg");
        let unsafe_svg = root.path().join("unsafe.svg");
        fs::write(&png, b"\x89PNG\r\n\x1a\n").unwrap();
        fs::write(&svg, b"<svg xmlns=\"http://www.w3.org/2000/svg\" />").unwrap();
        fs::write(&unsafe_svg, b"<svg><script>alert(1)</script></svg>").unwrap();

        let png = AbsolutePathBuf::try_from(png).unwrap();
        let svg = AbsolutePathBuf::try_from(svg).unwrap();
        let unsafe_svg = AbsolutePathBuf::try_from(unsafe_svg).unwrap();
        assert!(validate_marketplace_asset_file(&png, root.path()).is_some());
        assert!(validate_marketplace_asset_file(&svg, root.path()).is_some());
        assert!(validate_marketplace_asset_file(&unsafe_svg, root.path()).is_none());
    }

    #[test]
    fn rejects_assets_over_the_size_limit() {
        let root = tempdir().unwrap();
        let path = root.path().join("large.png");
        let mut bytes = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
        bytes.resize(MARKETPLACE_ASSET_MAX_BYTES as usize + 1, 0);
        fs::write(&path, bytes).unwrap();
        let path = AbsolutePathBuf::try_from(path).unwrap();
        assert!(validate_marketplace_asset_file(&path, root.path()).is_none());
    }
}
