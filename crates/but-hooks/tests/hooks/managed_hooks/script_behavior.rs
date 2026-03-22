//! Integration tests for the actual shell script behavior of managed hooks.
//!
//! These tests run the installed hook scripts in real git repositories
//! to verify runtime behavior that can't be tested at the filesystem level.

use std::process::Command;

use anyhow::Result;
use but_hooks::managed_hooks::install_managed_hooks;
use tempfile::TempDir;

/// Helper to run a git command in a directory, returning trimmed stdout.
fn git(repo_dir: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_dir)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run git {}: {e}", args.join(" ")));
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Create a temporary git repo with an initial commit on `main`, returning
/// the temp directory (kept alive by the caller) and the repo path.
fn init_test_repo() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let repo_dir = temp_dir.path().to_path_buf();
    git(&repo_dir, &["init", "-b", "main"]);
    git(&repo_dir, &["config", "user.email", "test@test.com"]);
    git(&repo_dir, &["config", "user.name", "Test"]);
    git(
        &repo_dir,
        &["commit", "--allow-empty", "-m", "initial commit"],
    );
    (temp_dir, repo_dir)
}

/// Regression test: branch names containing "gitbutler/workspace" as a substring
/// must NOT trigger post-checkout cleanup. Only the actual gitbutler/workspace
/// branch (or its ancestors via `~N`/`^N` notation) should trigger it.
#[test]
#[cfg(unix)]
fn post_checkout_ignores_branches_with_workspace_substring() -> Result<()> {
    let (_temp, repo_dir) = init_test_repo();
    let repo_dir = repo_dir.as_path();

    // Install managed hooks
    let hooks_dir = repo_dir.join(".git/hooks");
    install_managed_hooks(&hooks_dir, false)?;

    // Create a branch whose name contains "gitbutler/workspace" as a substring
    git(
        repo_dir,
        &["checkout", "-b", "feature/gitbutler/workspace-fix"],
    );
    git(
        repo_dir,
        &["commit", "--allow-empty", "-m", "feature commit"],
    );
    let feature_sha = git(repo_dir, &["rev-parse", "HEAD"]);

    // Switch back to main
    git(repo_dir, &["checkout", "main"]);
    let main_sha = git(repo_dir, &["rev-parse", "HEAD"]);

    // Run the post-checkout hook script directly
    let output = Command::new("sh")
        .arg(hooks_dir.join("post-checkout"))
        .args([&feature_sha, &main_sha, "1"])
        .current_dir(repo_dir)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // The hook must NOT trigger cleanup for a branch that merely contains
    // "gitbutler/workspace" as a substring
    assert!(
        !stdout.contains("Cleaning up GitButler hooks"),
        "Hook should not cleanup for branch 'feature/gitbutler/workspace-fix', got: {stdout}"
    );

    // All managed hooks should still be present
    super::assert_all_hooks_exist(&hooks_dir);

    Ok(())
}

#[test]
#[cfg(unix)]
fn post_checkout_does_not_clean_up_when_previous_state_was_detached() -> Result<()> {
    let (_temp, repo_dir) = init_test_repo();
    let repo_dir = repo_dir.as_path();

    git(repo_dir, &["checkout", "-b", "gitbutler/workspace"]);
    git(
        repo_dir,
        &["commit", "--allow-empty", "-m", "workspace commit"],
    );
    let workspace_sha = git(repo_dir, &["rev-parse", "HEAD"]);

    git(repo_dir, &["checkout", "--detach"]);
    git(repo_dir, &["checkout", "main"]);
    let main_sha = git(repo_dir, &["rev-parse", "HEAD"]);

    let hooks_dir = repo_dir.join(".git/hooks");
    install_managed_hooks(&hooks_dir, false)?;

    let output = Command::new("sh")
        .arg(hooks_dir.join("post-checkout"))
        .args([&workspace_sha, &main_sha, "1"])
        .current_dir(repo_dir)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !stdout.contains("Cleaning up GitButler hooks"),
        "Hook should not cleanup when previous checkout source was detached HEAD, got: {stdout}"
    );
    super::assert_all_hooks_exist(&hooks_dir);

    Ok(())
}

#[test]
#[cfg(unix)]
fn post_checkout_does_not_clean_up_when_previous_state_was_detached_at_workspace_tip() -> Result<()>
{
    let (_temp, repo_dir) = init_test_repo();
    let repo_dir = repo_dir.as_path();

    git(repo_dir, &["checkout", "-b", "gitbutler/workspace"]);
    git(
        repo_dir,
        &["commit", "--allow-empty", "-m", "workspace commit"],
    );
    let workspace_sha = git(repo_dir, &["rev-parse", "HEAD"]);

    git(repo_dir, &["checkout", "--detach", &workspace_sha]);
    git(repo_dir, &["checkout", "main"]);
    let main_sha = git(repo_dir, &["rev-parse", "HEAD"]);

    let hooks_dir = repo_dir.join(".git/hooks");
    install_managed_hooks(&hooks_dir, false)?;

    let output = Command::new("sh")
        .arg(hooks_dir.join("post-checkout"))
        .args([&workspace_sha, &main_sha, "1"])
        .current_dir(repo_dir)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !stdout.contains("Cleaning up GitButler hooks"),
        "Hook should not cleanup when previous checkout source was detached HEAD at workspace tip, got: {stdout}"
    );
    super::assert_all_hooks_exist(&hooks_dir);

    Ok(())
}

/// Verify that the post-checkout hook DOES trigger cleanup when leaving
/// the actual gitbutler/workspace branch (positive case for the grep pattern).
#[test]
#[cfg(unix)]
fn post_checkout_cleans_up_when_leaving_real_workspace() -> Result<()> {
    let (_temp, repo_dir) = init_test_repo();
    let repo_dir = repo_dir.as_path();

    // Create the real gitbutler/workspace branch
    git(repo_dir, &["checkout", "-b", "gitbutler/workspace"]);
    git(
        repo_dir,
        &["commit", "--allow-empty", "-m", "workspace commit"],
    );
    let workspace_sha = git(repo_dir, &["rev-parse", "HEAD"]);

    // Switch to main
    git(repo_dir, &["checkout", "main"]);
    let main_sha = git(repo_dir, &["rev-parse", "HEAD"]);

    // Install managed hooks
    let hooks_dir = repo_dir.join(".git/hooks");
    install_managed_hooks(&hooks_dir, false)?;

    // Run the post-checkout hook: leaving workspace → main
    let output = Command::new("sh")
        .arg(hooks_dir.join("post-checkout"))
        .args([&workspace_sha, &main_sha, "1"])
        .current_dir(repo_dir)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // The hook SHOULD trigger cleanup
    assert!(
        stdout.contains("Cleaning up GitButler hooks"),
        "Hook should cleanup when leaving gitbutler/workspace, got: {stdout}"
    );

    // Managed hooks should be removed by cleanup
    super::assert_no_hooks_exist(&hooks_dir);

    Ok(())
}
