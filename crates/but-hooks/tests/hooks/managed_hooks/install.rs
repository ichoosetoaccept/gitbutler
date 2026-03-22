use std::fs;

use anyhow::Result;
use but_hooks::managed_hooks::{HookInstallationResult, install_managed_hooks};
use tempfile::TempDir;

use super::{create_hooks_dir, create_user_hook, hook_exists, read_hook};

#[test]
fn creates_hooks_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let hooks_dir = temp_dir.path().join("hooks");
    // hooks_dir does not exist yet

    install_managed_hooks(&hooks_dir, false)?;

    assert!(hooks_dir.exists(), "Hooks directory should be created");
    Ok(())
}

#[test]
fn creates_pre_commit_post_checkout_and_pre_push() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    install_managed_hooks(&hooks_dir, false)?;

    assert!(
        hook_exists(&hooks_dir, "pre-commit"),
        "pre-commit hook should exist"
    );
    assert!(
        hook_exists(&hooks_dir, "post-checkout"),
        "post-checkout hook should exist"
    );
    assert!(
        hook_exists(&hooks_dir, "pre-push"),
        "pre-push hook should exist"
    );
    Ok(())
}

#[test]
fn hooks_have_gitbutler_signature() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    install_managed_hooks(&hooks_dir, false)?;

    let pre_commit = read_hook(&hooks_dir, "pre-commit")?;
    let post_checkout = read_hook(&hooks_dir, "post-checkout")?;
    let pre_push = read_hook(&hooks_dir, "pre-push")?;

    assert!(
        pre_commit.contains("GITBUTLER_MANAGED_HOOK_V1"),
        "pre-commit should have signature"
    );
    assert!(
        post_checkout.contains("GITBUTLER_MANAGED_HOOK_V1"),
        "post-checkout should have signature"
    );
    assert!(
        pre_push.contains("GITBUTLER_MANAGED_HOOK_V1"),
        "pre-push should have signature"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn hooks_are_executable() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    install_managed_hooks(&hooks_dir, false)?;

    assert!(
        super::is_executable(&hooks_dir, "pre-commit"),
        "pre-commit should be executable"
    );
    assert!(
        super::is_executable(&hooks_dir, "post-checkout"),
        "post-checkout should be executable"
    );
    assert!(
        super::is_executable(&hooks_dir, "pre-push"),
        "pre-push should be executable"
    );
    Ok(())
}

#[test]
fn is_idempotent() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    // Install twice
    let result1 = install_managed_hooks(&hooks_dir, false)?;
    let result2 = install_managed_hooks(&hooks_dir, false)?;

    // First install should succeed
    assert!(matches!(result1, HookInstallationResult::Success));

    // Second install should detect already configured
    assert!(matches!(result2, HookInstallationResult::AlreadyConfigured));

    // Hooks should still exist and be valid
    assert!(hook_exists(&hooks_dir, "pre-commit"));
    assert!(hook_exists(&hooks_dir, "post-checkout"));
    Ok(())
}

#[test]
fn preserves_external_hooks() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    let user_hook_content =
        "#!/bin/sh\n# User's custom pre-commit hook\necho 'Running user hook'\n";
    create_user_hook(&hooks_dir, "pre-commit", user_hook_content)?;

    // Install should skip pre-commit (external hook, no prior backup)
    // but still install post-checkout, so overall result is Success
    install_managed_hooks(&hooks_dir, false)?;

    // User hook should be untouched
    let content = read_hook(&hooks_dir, "pre-commit")?;
    assert_eq!(content, user_hook_content, "User hook should be preserved");

    // No backup should be created
    assert!(
        !hook_exists(&hooks_dir, "pre-commit-user"),
        "No backup should be created for external hooks"
    );

    // post-checkout (no existing hook) should still be installed
    assert!(
        hook_exists(&hooks_dir, "post-checkout"),
        "post-checkout should be installed when no external hook exists"
    );
    Ok(())
}

#[test]
fn does_not_overwrite_existing_backup() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    let original_backup = "#!/bin/sh\n# Original user hook\necho 'original'\n";
    let new_hook = "#!/bin/sh\n# New hook\necho 'new'\n";

    // Create original backup
    create_user_hook(&hooks_dir, "pre-commit-user", original_backup)?;

    // Create a new hook (not GitButler managed)
    create_user_hook(&hooks_dir, "pre-commit", new_hook)?;

    // Install GitButler hooks - should NOT overwrite the backup
    install_managed_hooks(&hooks_dir, false)?;

    // Backup should still have original content
    let backup_content = read_hook(&hooks_dir, "pre-commit-user")?;
    assert_eq!(
        backup_content, original_backup,
        "Backup should not be overwritten"
    );
    Ok(())
}

#[test]
fn into_custom_hooks_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // The caller resolves core.hooksPath — we just verify install_managed_hooks
    // correctly installs into whatever directory is passed.
    let custom_hooks = temp_dir.path().join("custom-hooks");
    let default_hooks = temp_dir.path().join("default-hooks");
    fs::create_dir_all(&default_hooks)?;

    // Install hooks into the custom directory
    install_managed_hooks(&custom_hooks, false)?;

    // Hooks should be in custom directory
    assert!(
        custom_hooks.join("pre-commit").exists(),
        "Hook should be in custom directory"
    );
    assert!(
        custom_hooks.join("post-checkout").exists(),
        "Hook should be in custom directory"
    );

    // Should NOT be in default directory
    assert!(
        !default_hooks.join("pre-commit").exists(),
        "Hook should not be in default location"
    );
    Ok(())
}

#[test]
fn partial_with_one_existing_hook() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    // Create only pre-commit user hook (external, not GitButler-managed)
    let user_hook = "#!/bin/sh\necho 'user pre-commit'\n";
    create_user_hook(&hooks_dir, "pre-commit", user_hook)?;

    // Install should preserve pre-commit (external) and create post-checkout
    install_managed_hooks(&hooks_dir, false)?;

    // pre-commit should be preserved (external hook, no backup created)
    let content = read_hook(&hooks_dir, "pre-commit")?;
    assert_eq!(
        content, user_hook,
        "External pre-commit should be preserved"
    );
    assert!(
        !hook_exists(&hooks_dir, "pre-commit-user"),
        "No backup should be created for external hooks"
    );

    // post-checkout should be newly installed
    assert!(hook_exists(&hooks_dir, "post-checkout"));
    let post = read_hook(&hooks_dir, "post-checkout")?;
    assert!(
        post.contains("GITBUTLER_MANAGED_HOOK_V1"),
        "post-checkout should be GitButler managed"
    );
    assert!(
        !hook_exists(&hooks_dir, "post-checkout-user"),
        "No backup for post-checkout"
    );

    // Uninstall should skip pre-commit (not managed), remove post-checkout
    but_hooks::managed_hooks::uninstall_managed_hooks(&hooks_dir)?;

    assert!(
        hook_exists(&hooks_dir, "pre-commit"),
        "External pre-commit should still exist"
    );
    assert!(
        !hook_exists(&hooks_dir, "post-checkout"),
        "post-checkout should be removed"
    );
    Ok(())
}

#[test]
fn preserves_hooks_with_shebang_variations() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    // Create external hooks with different shebangs
    let bash_env = "#!/usr/bin/env bash\necho 'bash hook'\n";
    let bash_direct = "#!/bin/bash\necho 'bash hook'\n";
    create_user_hook(&hooks_dir, "pre-commit", bash_env)?;
    create_user_hook(&hooks_dir, "post-checkout", bash_direct)?;

    // Install should preserve both external hooks
    install_managed_hooks(&hooks_dir, false)?;

    // Hooks should be untouched (no backups created)
    assert!(!hook_exists(&hooks_dir, "pre-commit-user"));
    assert!(!hook_exists(&hooks_dir, "post-checkout-user"));

    // Verify original shebangs are preserved
    let pre_restored = read_hook(&hooks_dir, "pre-commit")?;
    let post_restored = read_hook(&hooks_dir, "post-checkout")?;

    assert!(pre_restored.starts_with("#!/usr/bin/env bash"));
    assert!(post_restored.starts_with("#!/bin/bash"));
    Ok(())
}

#[test]
fn into_empty_hooks_directory() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    // hooks_dir exists but is empty (create_hooks_dir creates it)

    // Should install cleanly
    let result = install_managed_hooks(&hooks_dir, false)?;
    assert!(matches!(result, HookInstallationResult::Success));

    assert!(hook_exists(&hooks_dir, "pre-commit"));
    assert!(hook_exists(&hooks_dir, "post-checkout"));
    Ok(())
}

#[test]
fn concurrent_with_backup_present() -> Result<()> {
    let (_temp, hooks_dir) = create_hooks_dir()?;

    // Simulate a scenario where backup already exists (from previous install)
    let backup_content = "#!/bin/sh\necho 'original backup'\n";
    create_user_hook(&hooks_dir, "pre-commit-user", backup_content)?;

    // Create a new hook that's different
    let new_hook = "#!/bin/sh\necho 'new hook'\n";
    create_user_hook(&hooks_dir, "pre-commit", new_hook)?;

    // Install should not overwrite the existing backup
    install_managed_hooks(&hooks_dir, false)?;

    let backup = read_hook(&hooks_dir, "pre-commit-user")?;
    assert_eq!(
        backup, backup_content,
        "Existing backup should not be modified"
    );
    Ok(())
}

mod force {
    use std::fs;

    use anyhow::Result;
    use but_hooks::managed_hooks::{HookInstallationResult, install_managed_hooks};

    use super::super::{create_hooks_dir, create_user_hook, hook_exists, read_hook};

    #[test]
    fn overwrites_external_hooks() -> Result<()> {
        let (_temp, hooks_dir) = create_hooks_dir()?;

        let external_hook = "#!/bin/sh\n# External manager hook\necho 'external'\n";
        create_user_hook(&hooks_dir, "pre-commit", external_hook)?;

        // Without force, the external hook is preserved
        let result = install_managed_hooks(&hooks_dir, false)?;
        assert!(matches!(
            result,
            HookInstallationResult::Success | HookInstallationResult::Skipped { .. }
        ));
        let content = read_hook(&hooks_dir, "pre-commit")?;
        assert_eq!(
            content, external_hook,
            "External hook should be preserved without force"
        );
        assert!(
            !hook_exists(&hooks_dir, "pre-commit-user"),
            "No backup without force"
        );

        // With force, the external hook is backed up and overwritten
        let result = install_managed_hooks(&hooks_dir, true)?;
        assert!(matches!(result, HookInstallationResult::Success));

        // GitButler hook should now be installed
        let content = read_hook(&hooks_dir, "pre-commit")?;
        assert!(
            content.contains("GITBUTLER_MANAGED_HOOK_V1"),
            "GitButler hook should be installed after force"
        );

        // Original hook should be backed up
        assert!(
            hook_exists(&hooks_dir, "pre-commit-user"),
            "Backup should exist after force"
        );
        let backup = read_hook(&hooks_dir, "pre-commit-user")?;
        assert_eq!(
            backup, external_hook,
            "Backup should contain original hook content"
        );

        Ok(())
    }

    /// Regression test: after force-install backs up an external hook, if the external
    /// tool (e.g. prek) later overwrites the GB hook, a non-force `install_managed_hooks`
    /// must skip (not silently overwrite the new external hook).
    #[test]
    fn non_force_skips_external_hook_even_when_backup_exists() -> Result<()> {
        let (_temp, hooks_dir) = create_hooks_dir()?;

        let external_hook_v1 = "#!/bin/sh\n# External manager hook v1\necho 'external v1'\n";
        create_user_hook(&hooks_dir, "pre-commit", external_hook_v1)?;

        // Step 1: force-install backs up external hook and installs GB hook
        let result = install_managed_hooks(&hooks_dir, true)?;
        assert!(matches!(result, HookInstallationResult::Success));
        assert!(
            hook_exists(&hooks_dir, "pre-commit-user"),
            "Backup should exist after force-install"
        );

        // Step 2: external tool overwrites GB hook with its own (e.g. `prek install`)
        let external_hook_v2 = "#!/bin/sh\n# File generated by prek\nprek hook-impl pre-commit\n";
        fs::write(hooks_dir.join("pre-commit"), external_hook_v2)?;

        // Step 3: non-force install must NOT overwrite the external hook
        let result = install_managed_hooks(&hooks_dir, false)?;
        assert!(
            matches!(result, HookInstallationResult::Skipped { .. }),
            "Non-force install should skip when external hook exists (even with backup), got: {result:?}"
        );

        // Verify external hook is preserved
        let content = read_hook(&hooks_dir, "pre-commit")?;
        assert_eq!(
            content, external_hook_v2,
            "External hook should NOT be overwritten by non-force install"
        );

        Ok(())
    }
}

mod staleness {
    use std::fs;

    use anyhow::Result;
    use but_hooks::managed_hooks::{HookInstallationResult, install_managed_hooks};

    use super::super::{create_hooks_dir, hook_exists, read_hook};

    #[test]
    fn updates_stale_managed_hook() -> Result<()> {
        let (_temp, hooks_dir) = create_hooks_dir()?;

        // Install hooks normally
        install_managed_hooks(&hooks_dir, false)?;

        // Manually overwrite pre-commit with stale content (keeping the marker)
        let stale_content =
            "#!/bin/sh\n# GITBUTLER_MANAGED_HOOK_V1\n# Stale version of the hook\nexit 0\n";
        fs::write(hooks_dir.join("pre-commit"), stale_content)?;

        // Verify the content is stale
        let before = read_hook(&hooks_dir, "pre-commit")?;
        assert_eq!(before, stale_content, "Precondition: hook should be stale");

        // Re-install should detect staleness and update
        let result = install_managed_hooks(&hooks_dir, false)?;
        assert!(
            matches!(result, HookInstallationResult::Success),
            "Re-install of stale hook should return Success, got: {result:?}"
        );

        // Content should now match the current template
        let after = read_hook(&hooks_dir, "pre-commit")?;
        assert_ne!(
            after, stale_content,
            "Hook content should have been updated"
        );
        assert!(
            after.contains("GITBUTLER_MANAGED_HOOK_V1"),
            "Updated hook should still have the signature"
        );
        assert!(
            after.contains("Cannot commit directly to gitbutler/workspace"),
            "Updated hook should have the current pre-commit logic"
        );

        Ok(())
    }

    #[test]
    fn reports_correctly_when_some_stale() -> Result<()> {
        let (_temp, hooks_dir) = create_hooks_dir()?;

        // Install hooks normally
        install_managed_hooks(&hooks_dir, false)?;

        // Make only pre-push stale (keeping marker)
        let stale_content = "#!/bin/sh\n# GITBUTLER_MANAGED_HOOK_V1\n# Stale pre-push\nexit 0\n";
        fs::write(hooks_dir.join("pre-push"), stale_content)?;

        // Re-install: 2 hooks current, 1 stale → overall result should be Success
        let result = install_managed_hooks(&hooks_dir, false)?;
        assert!(
            matches!(result, HookInstallationResult::Success),
            "Should return Success when any hook was updated, got: {result:?}"
        );

        // All hooks should now have current content
        let pre_commit = read_hook(&hooks_dir, "pre-commit")?;
        let post_checkout = read_hook(&hooks_dir, "post-checkout")?;
        let pre_push = read_hook(&hooks_dir, "pre-push")?;

        assert!(pre_commit.contains("Cannot commit directly to gitbutler/workspace"));
        assert!(post_checkout.contains("You have left GitButler"));
        assert!(pre_push.contains("Cannot push the gitbutler/workspace"));
        assert_ne!(pre_push, stale_content, "pre-push should have been updated");

        Ok(())
    }

    #[test]
    fn reinstalls_missing_hooks() -> Result<()> {
        let (_temp, hooks_dir) = create_hooks_dir()?;

        // Install all hooks
        install_managed_hooks(&hooks_dir, false)?;
        assert!(hook_exists(&hooks_dir, "pre-commit"));
        assert!(hook_exists(&hooks_dir, "post-checkout"));
        assert!(hook_exists(&hooks_dir, "pre-push"));

        // User manually deletes post-checkout
        fs::remove_file(hooks_dir.join("post-checkout"))?;
        assert!(!hook_exists(&hooks_dir, "post-checkout"));

        // Re-install should reinstall the missing hook
        let result = install_managed_hooks(&hooks_dir, false)?;
        assert!(
            matches!(result, HookInstallationResult::Success),
            "Should return Success when reinstalling missing hook, got: {result:?}"
        );

        // All hooks should exist with current content
        assert!(hook_exists(&hooks_dir, "pre-commit"));
        assert!(hook_exists(&hooks_dir, "post-checkout"));
        assert!(hook_exists(&hooks_dir, "pre-push"));

        let post_checkout = read_hook(&hooks_dir, "post-checkout")?;
        assert!(
            post_checkout.contains("GITBUTLER_MANAGED_HOOK_V1"),
            "Reinstalled post-checkout should have the signature"
        );

        Ok(())
    }
}

mod roundtrip {
    use std::fs;

    use anyhow::Result;
    use but_hooks::managed_hooks::{
        HookInstallationResult, install_managed_hooks, uninstall_managed_hooks,
    };

    use super::super::{
        create_hooks_dir, create_managed_hook, create_user_hook, hook_exists, read_hook,
    };

    #[test]
    fn install_uninstall_with_user_hooks() -> Result<()> {
        let (_temp, hooks_dir) = create_hooks_dir()?;

        let original_pre_commit = "#!/bin/sh\n# Original pre-commit\necho 'pre-commit'\n";
        let original_post_checkout = "#!/bin/sh\n# Original post-checkout\necho 'post-checkout'\n";
        let original_pre_push = "#!/bin/sh\n# Original pre-push\necho 'pre-push'\n";

        // Simulate prior GitButler installation: managed hooks + user backups
        create_managed_hook(&hooks_dir, "pre-commit")?;
        create_managed_hook(&hooks_dir, "post-checkout")?;
        create_managed_hook(&hooks_dir, "pre-push")?;
        create_user_hook(&hooks_dir, "pre-commit-user", original_pre_commit)?;
        create_user_hook(&hooks_dir, "post-checkout-user", original_post_checkout)?;
        create_user_hook(&hooks_dir, "pre-push-user", original_pre_push)?;

        // Re-installing should either update stale hooks or detect them as current
        let result = install_managed_hooks(&hooks_dir, false)?;
        assert!(matches!(
            result,
            HookInstallationResult::Success | HookInstallationResult::AlreadyConfigured
        ));

        // Uninstall should restore originals
        uninstall_managed_hooks(&hooks_dir)?;

        // Verify original hooks are restored
        let restored_pre = read_hook(&hooks_dir, "pre-commit")?;
        let restored_post = read_hook(&hooks_dir, "post-checkout")?;
        let restored_push = read_hook(&hooks_dir, "pre-push")?;
        assert_eq!(
            restored_pre, original_pre_commit,
            "pre-commit should be restored"
        );
        assert_eq!(
            restored_post, original_post_checkout,
            "post-checkout should be restored"
        );
        assert_eq!(
            restored_push, original_pre_push,
            "pre-push should be restored"
        );

        // Verify backups are gone
        assert!(!hook_exists(&hooks_dir, "pre-commit-user"));
        assert!(!hook_exists(&hooks_dir, "post-checkout-user"));
        assert!(!hook_exists(&hooks_dir, "pre-push-user"));
        Ok(())
    }

    #[test]
    fn multiple_cycles() -> Result<()> {
        let (_temp, hooks_dir) = create_hooks_dir()?;

        let user_hook = "#!/bin/sh\necho 'user hook'\n";
        create_user_hook(&hooks_dir, "pre-commit", user_hook)?;

        // Cycle 1
        install_managed_hooks(&hooks_dir, false)?;
        uninstall_managed_hooks(&hooks_dir)?;

        // Cycle 2
        install_managed_hooks(&hooks_dir, false)?;
        uninstall_managed_hooks(&hooks_dir)?;

        // Cycle 3
        install_managed_hooks(&hooks_dir, false)?;
        uninstall_managed_hooks(&hooks_dir)?;

        // User hook should still be intact
        assert!(hook_exists(&hooks_dir, "pre-commit"));
        let content = read_hook(&hooks_dir, "pre-commit")?;
        assert_eq!(
            content, user_hook,
            "User hook should survive multiple cycles"
        );
        Ok(())
    }

    #[test]
    fn manually_modified_hook_after_install() -> Result<()> {
        let (_temp, hooks_dir) = create_hooks_dir()?;

        // Install GitButler hooks
        install_managed_hooks(&hooks_dir, false)?;

        // User manually modifies the hook
        let modified_hook = "#!/bin/sh\n# User modified this\necho 'modified'\n";
        let hook_path = hooks_dir.join("pre-commit");
        fs::write(&hook_path, modified_hook)?;

        // Uninstall should not remove the modified hook (no signature)
        uninstall_managed_hooks(&hooks_dir)?;

        // Hook should still exist with user's modifications
        assert!(hook_exists(&hooks_dir, "pre-commit"));
        let content = read_hook(&hooks_dir, "pre-commit")?;
        assert_eq!(content, modified_hook, "Modified hook should be preserved");
        Ok(())
    }
}
