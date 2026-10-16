//! Program resolution for MCP server execution on Unix-like systems.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;

/// Unix delegates PATH lookup and script execution to the kernel.
pub fn resolve(
    program: OsString,
    _env: &HashMap<OsString, OsString>,
    _cwd: &Path,
) -> std::io::Result<OsString> {
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::create_env_for_mcp_server;
    use anyhow::Result;
    use std::fs;
    use tempfile::TempDir;
    use tokio::process::Command;

    #[tokio::test]
    async fn resolved_program_executes_successfully() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let program = temp_dir.path().join("test_mcp_server");
        fs::write(&program, "#!/bin/sh\nexit 0")?;
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&program)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&program, permissions)?;

        let path = std::env::join_paths([temp_dir.path(), Path::new("/usr/bin")])?;
        let env =
            create_env_for_mcp_server(Some(HashMap::from([(OsString::from("PATH"), path)])), &[])?;
        let resolved = resolve(
            OsString::from("test_mcp_server"),
            &env,
            std::env::current_dir()?.as_path(),
        )?;
        assert!(
            Command::new(resolved)
                .envs(env)
                .output()
                .await?
                .status
                .success()
        );
        Ok(())
    }
}
