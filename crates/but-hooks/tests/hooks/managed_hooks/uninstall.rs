use anyhow::Result;
use but_hooks::managed_hooks::{
    HookInstallationResult, install_managed_hooks, uninstall_managed_hooks,
};

use super::{create_hooks_dir, create_managed_hook, create_user_hook, hook_exists, read_hook};

#[test]
fn removes_managed_hooks() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    install_managed_hooks(&hooks_dir, false)?;
    assert!(hook_exists(&hooks_dir, "pre-commit"));
    assert!(hook_exists(&hooks_dir, "post-checkout"));
    assert!(hook_exists(&hooks_dir, "pre-push"));

    uninstall_managed_hooks(&hooks_dir)?;

    assert!(
        !hook_exists(&hooks_dir, "pre-commit"),
        "pre-commit should be removed"
    );
    assert!(
        !hook_exists(&hooks_dir, "post-checkout"),
        "post-checkout should be removed"
    );
    assert!(
        !hook_exists(&hooks_dir, "pre-push"),
        "pre-push should be removed"
    );
    Ok(())
}

#[test]
fn restores_user_hooks() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    let user_hook_content = "#!/bin/sh\n# User's custom hook\necho 'user hook'\n";

    // Simulate prior GitButler installation: managed hook + user backup
    create_managed_hook(&hooks_dir, "pre-commit")?;
    create_user_hook(&hooks_dir, "pre-commit-user", user_hook_content)?;

    // Uninstall should restore the backup
    uninstall_managed_hooks(&hooks_dir)?;

    assert!(
        hook_exists(&hooks_dir, "pre-commit"),
        "User hook should be restored"
    );
    assert!(
        !hook_exists(&hooks_dir, "pre-commit-user"),
        "Backup should be removed after restore"
    );

    let restored_content = read_hook(&hooks_dir, "pre-commit")?;
    assert_eq!(
        restored_content, user_hook_content,
        "Restored hook should have original content"
    );
    Ok(())
}

#[test]
fn does_not_remove_non_managed_hooks() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    // Create a non-GitButler hook
    let user_hook = "#!/bin/sh\n# Not a GitButler hook\necho 'user hook'\n";
    create_user_hook(&hooks_dir, "pre-commit", user_hook)?;

    // Try to uninstall - should not remove the hook
    let result = uninstall_managed_hooks(&hooks_dir)?;

    // Hook should still exist
    assert!(
        hook_exists(&hooks_dir, "pre-commit"),
        "Non-managed hook should not be removed"
    );
    let content = read_hook(&hooks_dir, "pre-commit")?;
    assert_eq!(content, user_hook, "Hook content should be unchanged");

    // Should report skipped (external hook)
    assert!(matches!(
        result,
        HookInstallationResult::Success | HookInstallationResult::Skipped { .. }
    ));
    Ok(())
}

#[test]
fn is_idempotent() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    install_managed_hooks(&hooks_dir, false)?;

    // Uninstall twice
    let result1 = uninstall_managed_hooks(&hooks_dir)?;
    let result2 = uninstall_managed_hooks(&hooks_dir)?;

    // Both should succeed or report no work to do
    assert!(matches!(result1, HookInstallationResult::Success));
    assert!(matches!(
        result2,
        HookInstallationResult::Success | HookInstallationResult::AlreadyConfigured
    ));
    Ok(())
}
