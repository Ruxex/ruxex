use crate::PluginGitMode;
use crate::configure_trusted_git_repository;
use crate::marketplace::MarketplacePluginSource;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;
use thiserror::Error;
use url::Url;

const CACHE_DIRECTORY: &str = "marketplace-assets";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const MARKETPLACE_PLUGIN_MANIFEST_CANDIDATES: &[&str] = &[
    ".codex-plugin/plugin.json",
    "plugin.json",
    ".ruxex-plugin/plugin.json",
    ".claude-plugin/plugin.json",
    ".cursor-plugin/plugin.json",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommandPlan {
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedAssetRoot {
    pub root: AbsolutePathBuf,
    pub plugin_root: AbsolutePathBuf,
    pub revision: String,
}

#[derive(Debug, Error)]
pub enum GitAssetMaterializationError {
    #[error("Git asset materialization supports only Git marketplace sources")]
    UnsupportedSource,
    #[error("invalid Git asset path: {0}")]
    InvalidPath(String),
    #[error("invalid Git revision")]
    InvalidRevision,
    #[error("failed to prepare Git asset cache: {0}")]
    Io(#[from] io::Error),
    #[error("Git {operation} failed")]
    CommandFailed { operation: &'static str },
    #[error("Git {operation} timed out")]
    CommandTimedOut { operation: &'static str },
    #[error("Git returned no revision")]
    MissingRevision,
    #[error("Git asset cache metadata is invalid")]
    InvalidCacheMetadata,
    #[error("Git asset path is not absolute")]
    InvalidAbsolutePath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheMetadata {
    source_key: String,
    revision: String,
    subpath: String,
}

pub struct GitAssetMaterializer {
    ruxex_home: PathBuf,
    cache_root: PathBuf,
    git_binary: PathBuf,
    timeout: Duration,
}

impl GitAssetMaterializer {
    pub fn new(ruxex_home: &Path) -> Result<Self, GitAssetMaterializationError> {
        let cache_root = ruxex_home.join(CACHE_DIRECTORY);
        fs::create_dir_all(&cache_root)?;
        Ok(Self {
            ruxex_home: ruxex_home.to_path_buf(),
            cache_root,
            git_binary: PathBuf::from("git"),
            timeout: COMMAND_TIMEOUT,
        })
    }

    pub fn command_plan(
        source: &MarketplacePluginSource,
        requested_paths: &[String],
    ) -> Result<Vec<GitCommandPlan>, GitAssetMaterializationError> {
        let (url, subpath, ref_name, sha) = git_source_parts(source)?;
        let sparse_paths = sparse_paths(subpath, requested_paths)?;
        let mut plan = Vec::new();
        if sha.is_none() {
            plan.push(GitCommandPlan {
                args: vec![
                    "ls-remote".to_string(),
                    url.to_string(),
                    ref_name.unwrap_or("HEAD").to_string(),
                ],
            });
        }
        let mut clone = vec![
            "clone".to_string(),
            "--filter=blob:none".to_string(),
            "--depth=1".to_string(),
            "--no-checkout".to_string(),
        ];
        if let Some(ref_name) = ref_name
            && sha.is_none()
        {
            clone.push("--branch".to_string());
            clone.push(ref_name.to_string());
        }
        clone.push(url.to_string());
        clone.push("<staging-repository>".to_string());
        plan.push(GitCommandPlan { args: clone });
        plan.push(GitCommandPlan {
            args: sparse_checkout_args(&sparse_paths),
        });
        plan.push(GitCommandPlan {
            args: vec![
                "checkout".to_string(),
                "--detach".to_string(),
                sha.or(ref_name).unwrap_or("HEAD").to_string(),
            ],
        });
        Ok(plan)
    }

    pub fn cache_key_for(url: &str, revision: &str, subpath: &str) -> String {
        let mut digest = Sha256::new();
        digest.update(canonical_source_identity(url).as_bytes());
        digest.update([0]);
        digest.update(revision.as_bytes());
        digest.update([0]);
        digest.update(subpath.as_bytes());
        digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub fn materialize(
        &self,
        source: &MarketplacePluginSource,
        requested_paths: &[String],
    ) -> Result<MaterializedAssetRoot, GitAssetMaterializationError> {
        let (url, subpath, ref_name, sha) = git_source_parts(source)?;
        let revision = self.resolve_revision(url, ref_name, sha)?;
        let key = Self::cache_key_for(url, &revision, subpath);
        let entry = self.cache_root.join(&key);
        let sparse_paths = sparse_paths(subpath, requested_paths)?;
        let source_key = source_key(url);

        if entry.is_dir() {
            self.validate_cache_metadata(&entry, &source_key, &revision, subpath)?;
            self.expand_sparse_checkout(&entry.join("repo"), &sparse_paths)?;
        } else {
            self.create_cache_entry(
                &entry,
                url,
                ref_name,
                sha,
                &revision,
                subpath,
                &sparse_paths,
            )?;
        }

        let root = entry.join("repo").canonicalize()?;
        let plugin_root = root.join(subpath);
        let root = AbsolutePathBuf::try_from(root)
            .map_err(|_| GitAssetMaterializationError::InvalidAbsolutePath)?;
        let plugin_root = AbsolutePathBuf::try_from(plugin_root)
            .map_err(|_| GitAssetMaterializationError::InvalidAbsolutePath)?;
        Ok(MaterializedAssetRoot {
            root,
            plugin_root,
            revision,
        })
    }

    fn resolve_revision(
        &self,
        url: &str,
        ref_name: Option<&str>,
        sha: Option<&str>,
    ) -> Result<String, GitAssetMaterializationError> {
        if let Some(sha) = sha {
            if !is_full_sha(sha) {
                return Err(GitAssetMaterializationError::InvalidRevision);
            }
            return Ok(sha.to_string());
        }

        let reference = ref_name.unwrap_or("HEAD");
        let output = self.run_remote_git(&["ls-remote", url, reference], "ls-remote")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let revision = stdout
            .lines()
            .find_map(|line| line.split_once('\t').map(|(revision, _)| revision.trim()))
            .filter(|revision| is_full_sha(revision))
            .ok_or(GitAssetMaterializationError::MissingRevision)?;
        Ok(revision.to_string())
    }

    fn create_cache_entry(
        &self,
        entry: &Path,
        url: &str,
        ref_name: Option<&str>,
        sha: Option<&str>,
        revision: &str,
        subpath: &str,
        sparse_paths: &[String],
    ) -> Result<(), GitAssetMaterializationError> {
        fs::create_dir_all(&self.cache_root)?;
        let staging = tempfile::Builder::new()
            .prefix("marketplace-assets-")
            .tempdir_in(&self.cache_root)?;
        let repository = staging.path().join("repo");
        let mut clone_args = vec![
            "clone".to_string(),
            "--filter=blob:none".to_string(),
            "--depth=1".to_string(),
            "--no-checkout".to_string(),
        ];
        if let Some(ref_name) = ref_name
            && sha.is_none()
        {
            clone_args.extend(["--branch".to_string(), ref_name.to_string()]);
        }
        clone_args.extend([url.to_string(), repository.to_string_lossy().to_string()]);
        self.run_remote_git(&clone_args, "clone")?;
        self.expand_sparse_checkout(&repository, sparse_paths)?;
        let checkout_reference = sha.or(ref_name).unwrap_or("HEAD");
        self.run_local_git(
            &repository,
            &["checkout", "--detach", checkout_reference],
            "checkout",
        )?;
        let metadata = CacheMetadata {
            source_key: source_key(url),
            revision: revision.to_string(),
            subpath: subpath.to_string(),
        };
        fs::write(
            staging.path().join("metadata.json"),
            serde_json::to_vec(&metadata).map_err(io::Error::other)?,
        )?;
        if entry.exists() {
            return Ok(());
        }
        fs::rename(staging.path(), entry)?;
        Ok(())
    }

    fn validate_cache_metadata(
        &self,
        entry: &Path,
        source_key: &str,
        revision: &str,
        subpath: &str,
    ) -> Result<(), GitAssetMaterializationError> {
        let metadata: CacheMetadata =
            serde_json::from_slice(&fs::read(entry.join("metadata.json"))?)
                .map_err(|_| GitAssetMaterializationError::InvalidCacheMetadata)?;
        if metadata.source_key != source_key
            || metadata.revision != revision
            || metadata.subpath != subpath
            || !entry.join("repo/.git").exists()
        {
            return Err(GitAssetMaterializationError::InvalidCacheMetadata);
        }
        Ok(())
    }

    fn expand_sparse_checkout(
        &self,
        repository: &Path,
        sparse_paths: &[String],
    ) -> Result<(), GitAssetMaterializationError> {
        let args = sparse_checkout_args(sparse_paths);
        self.run_local_git(repository, &args, "sparse-checkout")?;
        Ok(())
    }

    fn run_remote_git(
        &self,
        args: &[impl AsRef<str>],
        operation: &'static str,
    ) -> Result<Output, GitAssetMaterializationError> {
        let mut command = PluginGitMode::Automatic.command(&self.git_binary);
        command.args(args.iter().map(|arg| arg.as_ref()));
        let _trusted_repository = configure_trusted_git_repository(&mut command, &self.ruxex_home)
            .map_err(|_| GitAssetMaterializationError::CommandFailed { operation })?;
        run_command_with_timeout(&mut command, operation, self.timeout)
    }

    fn run_local_git(
        &self,
        repository: &Path,
        args: &[impl AsRef<str>],
        operation: &'static str,
    ) -> Result<Output, GitAssetMaterializationError> {
        let mut command = PluginGitMode::Automatic.command(&self.git_binary);
        command.current_dir(repository);
        command.args(args.iter().map(|arg| arg.as_ref()));
        run_command_with_timeout(&mut command, operation, self.timeout)
    }
}

fn git_source_parts(
    source: &MarketplacePluginSource,
) -> Result<(&str, &str, Option<&str>, Option<&str>), GitAssetMaterializationError> {
    match source {
        MarketplacePluginSource::Git {
            url,
            path,
            ref_name,
            sha,
        } => {
            validate_relative_path(path.as_deref().unwrap_or(""))?;
            if let Some(sha) = sha
                && !is_full_sha(sha)
            {
                return Err(GitAssetMaterializationError::InvalidRevision);
            }
            Ok((
                url,
                path.as_deref().unwrap_or(""),
                ref_name.as_deref(),
                sha.as_deref(),
            ))
        }
        _ => Err(GitAssetMaterializationError::UnsupportedSource),
    }
}

fn sparse_paths(
    subpath: &str,
    requested_paths: &[String],
) -> Result<Vec<String>, GitAssetMaterializationError> {
    let mut paths = MARKETPLACE_PLUGIN_MANIFEST_CANDIDATES
        .iter()
        .map(|candidate| join_repo_path(subpath, candidate))
        .collect::<Result<Vec<_>, _>>()?;
    for requested_path in requested_paths {
        validate_relative_path(requested_path)?;
        paths.push(join_repo_path(subpath, requested_path)?);
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn sparse_checkout_args(paths: &[String]) -> Vec<String> {
    let mut args = vec![
        "sparse-checkout".to_string(),
        "set".to_string(),
        "--no-cone".to_string(),
        "--".to_string(),
    ];
    args.extend(paths.iter().cloned());
    args
}

fn join_repo_path(subpath: &str, path: &str) -> Result<String, GitAssetMaterializationError> {
    validate_relative_path(subpath)?;
    validate_relative_path(path)?;
    if subpath.is_empty() {
        Ok(path.to_string())
    } else if path.is_empty() {
        Ok(subpath.to_string())
    } else {
        Ok(format!("{subpath}/{path}"))
    }
}

fn validate_relative_path(path: &str) -> Result<(), GitAssetMaterializationError> {
    if path.is_empty() {
        return Ok(());
    }
    if path.starts_with('/') || path.contains('\\') {
        return Err(GitAssetMaterializationError::InvalidPath(path.to_string()));
    }
    let mut saw_segment = false;
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(GitAssetMaterializationError::InvalidPath(path.to_string()));
        }
        saw_segment = true;
    }
    saw_segment
        .then_some(())
        .ok_or_else(|| GitAssetMaterializationError::InvalidPath(path.to_string()))
}

fn is_full_sha(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn canonical_source_identity(source: &str) -> String {
    if let Ok(mut url) = Url::parse(source) {
        let _ = url.set_username("");
        let _ = url.set_password(None);
        url.set_fragment(None);
        url.to_string()
    } else {
        Path::new(source)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(source))
            .to_string_lossy()
            .into_owned()
    }
}

fn source_key(source: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(canonical_source_identity(source).as_bytes());
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn run_command_with_timeout(
    command: &mut Command,
    operation: &'static str,
    timeout: Duration,
) -> Result<Output, GitAssetMaterializationError> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| GitAssetMaterializationError::CommandFailed { operation })?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|_| GitAssetMaterializationError::CommandFailed { operation })?;
                if status.success() {
                    return Ok(output);
                }
                return Err(GitAssetMaterializationError::CommandFailed { operation });
            }
            Ok(None) if start.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitAssetMaterializationError::CommandTimedOut { operation });
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitAssetMaterializationError::CommandFailed { operation });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketplace::MarketplacePluginSource;
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn plans_a_filtered_shallow_sparse_materialization_without_full_clone_fallback() {
        let source = MarketplacePluginSource::Git {
            url: "https://example.com/catalog.git".to_string(),
            path: Some("plugins/weather".to_string()),
            ref_name: None,
            sha: Some("0123456789012345678901234567890123456789".to_string()),
        };

        let plan = GitAssetMaterializer::command_plan(&source, &["assets/logo.svg".to_string()])
            .expect("Git source should produce a plan");

        let clone = plan
            .iter()
            .find(|command| command.args.contains(&"clone".to_string()))
            .unwrap();
        assert!(clone.args.iter().any(|arg| arg == "--filter=blob:none"));
        assert!(clone.args.iter().any(|arg| arg == "--depth=1"));
        assert!(clone.args.iter().any(|arg| arg == "--no-checkout"));
        assert!(plan.iter().any(|command| {
            command
                .args
                .windows(2)
                .any(|args| args == ["sparse-checkout", "set"])
                && command
                    .args
                    .iter()
                    .any(|arg| arg == "plugins/weather/assets/logo.svg")
                && command
                    .args
                    .iter()
                    .any(|arg| arg == "plugins/weather/.codex-plugin/plugin.json")
        }));
        assert!(!plan.iter().any(|command| {
            command.args.iter().any(|arg| arg == "clone")
                && !command.args.iter().any(|arg| arg == "--filter=blob:none")
        }));
    }

    #[test]
    fn cache_key_separates_url_revision_and_plugin_subpath() {
        let first = GitAssetMaterializer::cache_key_for(
            "https://example.com/catalog.git",
            "0123456789012345678901234567890123456789",
            "plugins/weather",
        );
        let same = GitAssetMaterializer::cache_key_for(
            "https://example.com/catalog.git",
            "0123456789012345678901234567890123456789",
            "plugins/weather",
        );
        let different_revision = GitAssetMaterializer::cache_key_for(
            "https://example.com/catalog.git",
            "abcdefabcdefabcdefabcdefabcdefabcdefabcd",
            "plugins/weather",
        );
        let different_subpath = GitAssetMaterializer::cache_key_for(
            "https://example.com/catalog.git",
            "0123456789012345678901234567890123456789",
            "plugins/other",
        );

        assert_eq!(first, same);
        assert_ne!(first, different_revision);
        assert_ne!(first, different_subpath);
    }

    #[test]
    fn reuses_a_materialized_cache_entry_for_identical_source_identity() {
        let root = tempdir().unwrap();
        let repository = root.path().join("repository");
        fs::create_dir_all(repository.join("assets")).unwrap();
        fs::write(repository.join("assets/logo.svg"), b"<svg />").unwrap();
        run_git(&repository, &["init", "--initial-branch=main"]);
        run_git(&repository, &["config", "user.email", "test@example.com"]);
        run_git(&repository, &["config", "user.name", "Test"]);
        run_git(&repository, &["add", "."]);
        run_git(&repository, &["commit", "-m", "fixture"]);
        let revision = git_output(&repository, &["rev-parse", "HEAD"]);

        let ruxex_home = root.path().join("ruxex-home");
        fs::create_dir_all(&ruxex_home).unwrap();
        let source = MarketplacePluginSource::Git {
            url: format!("file://{}", repository.display()),
            path: None,
            ref_name: None,
            sha: Some(revision),
        };
        let materializer = GitAssetMaterializer::new(&ruxex_home).unwrap();
        let first = materializer
            .materialize(&source, &["assets/logo.svg".to_string()])
            .unwrap();
        let second = materializer
            .materialize(&source, &["assets/logo.svg".to_string()])
            .unwrap();

        assert_eq!(first, second);
        assert!(
            first
                .plugin_root
                .as_path()
                .join("assets/logo.svg")
                .is_file()
        );
        assert_eq!(
            fs::read_dir(ruxex_home.join("marketplace-assets"))
                .unwrap()
                .count(),
            1
        );
    }

    fn run_git(repository: &std::path::Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repository)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_output(repository: &std::path::Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(repository)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }
}
