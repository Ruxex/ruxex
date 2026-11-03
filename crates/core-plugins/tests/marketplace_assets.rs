use ruxex_core_plugins::GitAssetMaterializationError;
use ruxex_core_plugins::GitAssetMaterializer;
use ruxex_core_plugins::MarketplaceAssetSourceKind;
use ruxex_core_plugins::classify_marketplace_asset_source;
use ruxex_core_plugins::marketplace::MarketplacePluginSource;

#[test]
fn git_asset_plans_are_bounded_and_source_discriminated() {
    let source = MarketplacePluginSource::Git {
        url: "https://git.example.test/catalog".to_string(),
        path: Some("plugins/weather".to_string()),
        ref_name: None,
        sha: Some("0123456789012345678901234567890123456789".to_string()),
    };
    assert_eq!(
        classify_marketplace_asset_source(&source),
        MarketplaceAssetSourceKind::Git
    );
    let plan = GitAssetMaterializer::command_plan(&source, &["assets/logo.svg".to_string()])
        .expect("Git source should produce a bounded plan");
    assert!(plan.iter().any(|command| {
        command.args.iter().any(|arg| arg == "--filter=blob:none")
            && command.args.iter().any(|arg| arg == "--depth=1")
            && command.args.iter().any(|arg| arg == "--no-checkout")
    }));
    assert!(plan.iter().any(|command| {
        command
            .args
            .iter()
            .any(|arg| arg == "plugins/weather/assets/logo.svg")
    }));
}

#[test]
fn npm_sources_are_not_treated_as_git_assets() {
    let source = MarketplacePluginSource::Npm {
        package: "weather".to_string(),
        version: None,
        registry: None,
    };
    assert_eq!(
        classify_marketplace_asset_source(&source),
        MarketplaceAssetSourceKind::Npm
    );
    assert!(matches!(
        GitAssetMaterializer::command_plan(&source, &[]),
        Err(GitAssetMaterializationError::UnsupportedSource)
    ));
}
