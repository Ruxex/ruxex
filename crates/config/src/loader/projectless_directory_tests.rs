//! Projectless classification must not persist trust for arbitrary directories.

use super::*;
use crate::NoopThreadConfigLoader;
use crate::loader::tests::TestFileSystem;
use pretty_assertions::assert_eq;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    home: AbsolutePathBuf,
    cwd: AbsolutePathBuf,
    overrides: LoaderOverrides,
}

impl Fixture {
    fn new() -> anyhow::Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = AbsolutePathBuf::from_absolute_path(temp.path().canonicalize()?)?;
        let home = root.join("home");
        let cwd = root.join("workspace");
        std::fs::create_dir_all(&home)?;
        std::fs::create_dir_all(&cwd)?;
        Ok(Self {
            _temp: temp,
            home,
            cwd,
            overrides: LoaderOverrides::without_managed_config_for_tests(),
        })
    }

    async fn load(&self) -> anyhow::Result<ConfigLayerStack> {
        Ok(load_config_layers_state(
            &TestFileSystem,
            self.home.as_path(),
            Some(self.cwd.clone()),
            &[],
            self.overrides.clone(),
            &NoopThreadConfigLoader,
        )
        .await?)
    }
}

#[tokio::test]
async fn unmarked_directory_is_projectless() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    assert!(fixture.load().await?.is_projectless());
    Ok(())
}

#[tokio::test]
async fn project_marker_prevents_projectless_classification() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    std::fs::create_dir(fixture.cwd.join(".git"))?;
    std::fs::write(fixture.cwd.join(".git/HEAD"), "ref: refs/heads/main\n")?;
    assert!(!fixture.load().await?.is_projectless());
    Ok(())
}

#[tokio::test]
async fn skipped_discovery_does_not_claim_projectless() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let layers = load_config_layers_state(
        &TestFileSystem,
        fixture.home.as_path(),
        Some(fixture.cwd),
        &[],
        LoaderOverrides {
            ignore_project_config: true,
            ..fixture.overrides
        },
        &NoopThreadConfigLoader,
    )
    .await?;
    assert!(!layers.is_projectless());
    Ok(())
}

#[tokio::test]
async fn find_project_root_keeps_cwd_fallback() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    assert_eq!(
        find_project_root(&TestFileSystem, &fixture.cwd, &[".company-root".into()]).await?,
        fixture.cwd
    );
    Ok(())
}
