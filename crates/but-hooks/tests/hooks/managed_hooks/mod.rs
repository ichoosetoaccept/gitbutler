mod config;
mod ensure;
mod install;
mod script_behavior;
mod uninstall;

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tempfile::TempDir;

/// Helper to create a test hooks directory (simulates `.git/hooks/`)
fn create_hooks_dir() -> Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let hooks_dir = temp_dir.path().join("hooks");
    fs::create_dir_all(&hooks_dir)?;
    Ok((temp_dir, hooks_dir))
}

/// Helper to create a gix repo with its hooks directory for `ensure_managed_hooks` tests.
fn create_repo_with_hooks_dir() -> Result<(TempDir, gix::Repository, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let repo = gix::init(temp_dir.path())?;
    let hooks_dir = repo.git_dir().join("hooks");
    fs::create_dir_all(&hooks_dir)?;
    Ok((temp_dir, repo, hooks_dir))
}

/// Helper to create a user hook file with content
fn create_user_hook(hooks_dir: &Path, hook_name: &str, content: &str) -> Result<()> {
    fs::create_dir_all(hooks_dir)?;
    let hook_path = hooks_dir.join(hook_name);
    fs::write(&hook_path, content)?;

    #[cfg(unix)]
    fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755))?;

    Ok(())
}

/// Helper to check if a file exists
fn hook_exists(hooks_dir: &Path, hook_name: &str) -> bool {
    hooks_dir.join(hook_name).exists()
}

/// Helper to read hook content
fn read_hook(hooks_dir: &Path, hook_name: &str) -> Result<String> {
    let path = hooks_dir.join(hook_name);
    Ok(fs::read_to_string(path)?)
}

/// Helper to create a GitButler-managed hook file directly (simulates prior installation)
fn create_managed_hook(hooks_dir: &Path, hook_name: &str) -> Result<()> {
    let content = format!(
        "#!/bin/sh\n# GITBUTLER_MANAGED_HOOK_V1\n# Test managed hook for {hook_name}\nexit 0\n"
    );
    create_user_hook(hooks_dir, hook_name, &content)
}

/// Helper to check if hook is executable on Unix
#[cfg(unix)]
fn is_executable(hooks_dir: &Path, hook_name: &str) -> bool {
    let path = hooks_dir.join(hook_name);
    if let Ok(metadata) = fs::metadata(&path) {
        let permissions = metadata.permissions();
        permissions.mode() & 0o111 != 0
    } else {
        false
    }
}
