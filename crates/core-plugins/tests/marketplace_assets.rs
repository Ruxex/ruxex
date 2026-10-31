use ruxex_core_plugins::MarketplaceAssetPaths;
use ruxex_core_plugins::MarketplaceAssetUrls;
use ruxex_core_plugins::MarketplaceOrigin;
use ruxex_core_plugins::marketplace::MarketplacePluginSource;
use ruxex_core_plugins::parse_marketplace_manifest_asset_paths;
use ruxex_core_plugins::resolve_marketplace_asset_urls;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use std::path::Path;

fn git_origin(revision: Option<&str>) -> MarketplaceOrigin {
    MarketplaceOrigin::Git {
        url: "https://github.com/acme/catalog.git".to_string(),
        ref_name: Some("main".to_string()),
        revision: revision.map(str::to_string),
    }
}

fn asset_paths(path: &str) -> MarketplaceAssetPaths {
    MarketplaceAssetPaths {
        composer_icon: None,
        logo: Some(path.to_string()),
        logo_dark: None,
        screenshots: Vec::new(),
    }
}

#[test]
fn resolves_external_git_plugin_assets_at_explicit_sha() {
    let source = MarketplacePluginSource::Git {
        url: "https://github.com/acme/plugin.git".to_string(),
        path: Some("plugins/weather".to_string()),
        ref_name: Some("release".to_string()),
        sha: Some("plugin-sha".to_string()),
    };

    let urls = resolve_marketplace_asset_urls(
        &git_origin(Some("marketplace-sha")),
        Path::new("/tmp/catalog"),
        &source,
        &asset_paths("assets/logo.svg"),
    )
    .expect("supported GitHub source should resolve");

    assert_eq!(
        urls,
        MarketplaceAssetUrls {
            composer_icon_url: None,
            logo_url: Some(
                "https://raw.githubusercontent.com/acme/plugin/plugin-sha/plugins/weather/assets/logo.svg"
                    .to_string(),
            ),
            logo_url_dark: None,
            screenshot_urls: Vec::new(),
        }
    );
}

#[test]
fn uses_plugin_ref_for_external_sources_and_head_without_one() {
    let source_with_ref = MarketplacePluginSource::Git {
        url: "https://github.com/acme/plugin".to_string(),
        path: None,
        ref_name: Some("plugin-ref".to_string()),
        sha: None,
    };
    let source_without_ref = MarketplacePluginSource::Git {
        url: "acme/plugin".to_string(),
        path: None,
        ref_name: None,
        sha: None,
    };

    let plugin_ref_urls = resolve_marketplace_asset_urls(
        &git_origin(Some("marketplace-sha")),
        Path::new("/tmp/catalog"),
        &source_with_ref,
        &asset_paths("logo.png"),
    )
    .expect("plugin ref should resolve")
    .logo_url
    .unwrap();
    assert!(plugin_ref_urls.contains("/plugin/plugin-ref/logo.png"));

    let head_urls = resolve_marketplace_asset_urls(
        &git_origin(Some("marketplace-sha")),
        Path::new("/tmp/catalog"),
        &source_without_ref,
        &asset_paths("logo.png"),
    )
    .expect("marketplace revision should resolve")
    .logo_url
    .unwrap();
    assert!(head_urls.contains("/plugin/HEAD/logo.png"));

    let marketplace_revision_urls = resolve_marketplace_asset_urls(
        &MarketplaceOrigin::Git {
            url: "https://github.com/acme/catalog.git".to_string(),
            ref_name: Some("main".to_string()),
            revision: Some("marketplace-sha".to_string()),
        },
        Path::new("/tmp/catalog"),
        &MarketplacePluginSource::Local {
            path: AbsolutePathBuf::try_from("/tmp/catalog/plugins/weather").unwrap(),
        },
        &asset_paths("logo.png"),
    )
    .expect("local source should inherit marketplace revision")
    .logo_url
    .unwrap();
    assert!(
        marketplace_revision_urls.contains("/catalog/marketplace-sha/plugins/weather/logo.png")
    );
}

#[test]
fn accepts_nested_branch_names_and_canonical_github_urls() {
    for repository in [
        "https://github.com/acme/plugin",
        "https://github.com/acme/plugin.git",
        "https://github.com/acme/plugin/",
        "https://github.com/acme/plugin.git/",
        "acme/plugin",
        "acme/plugin.git",
    ] {
        let urls = resolve_marketplace_asset_urls(
            &git_origin(None),
            Path::new("/tmp/catalog"),
            &MarketplacePluginSource::Git {
                url: repository.to_string(),
                path: None,
                ref_name: Some("feature/ui".to_string()),
                sha: None,
            },
            &asset_paths("assets/logo.svg"),
        )
        .expect("valid GitHub repository and branch should resolve");

        assert_eq!(
            urls.logo_url.as_deref(),
            Some("https://raw.githubusercontent.com/acme/plugin/feature/ui/assets/logo.svg"),
            "repository form should be canonicalized: {repository}"
        );
    }
}

#[test]
fn resolves_local_plugin_only_inside_git_marketplace_checkout() {
    let source = MarketplacePluginSource::Local {
        path: AbsolutePathBuf::try_from("/tmp/catalog/plugins/weather").unwrap(),
    };

    let urls = resolve_marketplace_asset_urls(
        &git_origin(Some("catalog-sha")),
        Path::new("/tmp/catalog"),
        &source,
        &asset_paths("assets/logo.svg"),
    )
    .expect("in-checkout local plugin should resolve")
    .logo_url
    .unwrap();
    assert_eq!(
        urls,
        "https://raw.githubusercontent.com/acme/catalog/catalog-sha/plugins/weather/assets/logo.svg"
    );

    assert!(
        resolve_marketplace_asset_urls(
            &MarketplaceOrigin::Local {
                path: AbsolutePathBuf::try_from("/tmp/catalog").unwrap(),
            },
            Path::new("/tmp/catalog"),
            &source,
            &asset_paths("assets/logo.svg"),
        )
        .is_none()
    );
}

#[test]
fn rejects_unsafe_paths_and_unsupported_origins() {
    let source = MarketplacePluginSource::Git {
        url: "https://github.com/acme/plugin".to_string(),
        path: Some("plugins/weather".to_string()),
        ref_name: None,
        sha: None,
    };

    for unsafe_path in [
        "../secret.svg",
        "plugins/../secret.svg",
        r"assets\logo.svg",
        "%2e%2e/secret.svg",
        "/absolute/logo.svg",
        "",
    ] {
        assert_eq!(
            resolve_marketplace_asset_urls(
                &git_origin(Some("sha")),
                Path::new("/tmp/catalog"),
                &source,
                &asset_paths(unsafe_path),
            )
            .and_then(|urls| urls.logo_url),
            None,
            "path should be rejected: {unsafe_path}"
        );
    }

    let arbitrary_origin = MarketplaceOrigin::Git {
        url: "https://example.com/acme/catalog.git".to_string(),
        ref_name: None,
        revision: Some("sha".to_string()),
    };
    let arbitrary_source = MarketplacePluginSource::Git {
        url: "https://example.com/acme/plugin".to_string(),
        path: None,
        ref_name: None,
        sha: None,
    };
    assert!(
        resolve_marketplace_asset_urls(
            &arbitrary_origin,
            Path::new("/tmp/catalog"),
            &arbitrary_source,
            &asset_paths("logo.svg"),
        )
        .is_none()
    );
}

#[test]
fn rejects_unsafe_github_urls_and_git_references() {
    for repository in [
        "",
        "http://github.com/acme/plugin",
        "https://github.com.evil/acme/plugin",
        "https://user@github.com/acme/plugin",
        "https://github.com:444/acme/plugin",
        "https://github.com/acme/plugin?ref=main",
        "https://github.com/acme/plugin#main",
        "https://github.com/acme/plugin/extra",
        "https://github.com/acme/plugin%2Fextra",
        "acme/plugin/extra",
        "acme//plugin",
    ] {
        let result = resolve_marketplace_asset_urls(
            &git_origin(None),
            Path::new("/tmp/catalog"),
            &MarketplacePluginSource::Git {
                url: repository.to_string(),
                path: None,
                ref_name: Some("main".to_string()),
                sha: None,
            },
            &asset_paths("assets/logo.svg"),
        );
        assert!(
            result.is_none(),
            "repository should be rejected: {repository}"
        );
    }

    for reference in [
        "",
        "/main",
        "main/",
        "main//other",
        "main/../other",
        "main..other",
        "main?download=1",
        "main#fragment",
        "main%2Fother",
        r"main\other",
        "main:other",
        "main~1",
        "main^1",
        "main*",
        "main[1]",
        "main @{other}",
    ] {
        let result = resolve_marketplace_asset_urls(
            &git_origin(None),
            Path::new("/tmp/catalog"),
            &MarketplacePluginSource::Git {
                url: "acme/plugin".to_string(),
                path: None,
                ref_name: Some(reference.to_string()),
                sha: None,
            },
            &asset_paths("assets/logo.svg"),
        );
        assert!(
            result.is_none(),
            "Git reference should be rejected: {reference}"
        );
    }
}

#[test]
fn parses_only_interface_asset_paths_from_remote_manifest() {
    let paths = parse_marketplace_manifest_asset_paths(
        br#"{
            "name": "weather",
            "interface": {
                "logo": "assets/logo.svg",
                "logoDark": "assets/logo-dark.svg",
                "composerIcon": "assets/icon.png",
                "screenshots": ["assets/one.png", "assets/two.svg"]
            },
            "mcpServers": {"server": {"command": "must not be interpreted"}},
            "hooks": ["must not be interpreted"]
        }"#,
    )
    .expect("valid manifest JSON should parse");

    assert_eq!(paths.logo.as_deref(), Some("assets/logo.svg"));
    assert_eq!(paths.logo_dark.as_deref(), Some("assets/logo-dark.svg"));
    assert_eq!(paths.composer_icon.as_deref(), Some("assets/icon.png"));
    assert_eq!(paths.screenshots, ["assets/one.png", "assets/two.svg"]);
}
