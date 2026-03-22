use super::*;
use std::fs;
use tempfile::TempDir;

/// When multiple hooks are externally owned, `install_managed_hooks(force=false)`
/// returns `Skipped` carrying the *first* skipped hook name. This is intentional:
/// callers treat it as an all-or-nothing signal (any external → switch mode).
/// `but hook status` enumerates each hook individually so no per-hook info is lost.
#[test]
fn install_reports_first_skipped_hook_when_multiple_are_externally_owned() {
    let dir = TempDir::new().unwrap();
    let hooks_dir = dir.path().join("hooks");
    fs::create_dir(&hooks_dir).unwrap();

    // Place non-GitButler hooks in two of the three managed hook slots.
    fs::write(
        hooks_dir.join("pre-commit"),
        "#!/bin/sh\necho 'external hook'\n",
    )
    .unwrap();
    fs::write(
        hooks_dir.join("post-checkout"),
        "#!/bin/sh\necho 'another external hook'\n",
    )
    .unwrap();

    let result = install_managed_hooks(&hooks_dir, false).unwrap();
    match result {
        HookInstallationResult::PartialSuccess { ref warnings, .. } => {
            // pre-push installs successfully, but pre-commit and post-checkout
            // are skipped — mixed result is PartialSuccess with skip warnings.
            assert_eq!(
                warnings.len(),
                2,
                "should have 2 skip warnings: {warnings:?}"
            );
            assert!(
                warnings.iter().any(|w| w.contains("pre-commit")),
                "should mention skipped pre-commit"
            );
            assert!(
                warnings.iter().any(|w| w.contains("post-checkout")),
                "should mention skipped post-checkout"
            );
        }
        other => panic!("expected PartialSuccess, got {other:?}"),
    }

    // Verify neither external hook was overwritten.
    let pre_commit = fs::read_to_string(hooks_dir.join("pre-commit")).unwrap();
    assert!(
        !pre_commit.contains(GITBUTLER_HOOK_SIGNATURE),
        "external pre-commit should NOT be overwritten"
    );
    let post_checkout = fs::read_to_string(hooks_dir.join("post-checkout")).unwrap();
    assert!(
        !post_checkout.contains(GITBUTLER_HOOK_SIGNATURE),
        "external post-checkout should NOT be overwritten"
    );
}

/// When `install_managed_hooks` returns `PartialSuccess` (some hooks skipped,
/// some installed), each skip warning must include the `--force-hooks` hint so
/// users know how to override the skip.
#[test]
fn partial_success_skip_warnings_include_force_hooks_hint() {
    let dir = TempDir::new().unwrap();
    let hooks_dir = dir.path().join("hooks");
    fs::create_dir(&hooks_dir).unwrap();

    // A plain user pre-commit hook — will be skipped.
    fs::write(hooks_dir.join("pre-commit"), "#!/bin/sh\necho 'my hook'\n").unwrap();

    let result = install_managed_hooks(&hooks_dir, false).unwrap();
    match result {
        HookInstallationResult::PartialSuccess { ref warnings, .. } => {
            let w = warnings
                .iter()
                .find(|w| w.contains("pre-commit"))
                .expect("should have a pre-commit skip warning");
            assert!(
                w.contains("--force-hooks"),
                "skip warning should include --force-hooks hint, got: {w:?}"
            );
        }
        other => panic!("expected PartialSuccess, got {other:?}"),
    }
}

/// `install_hook(force=false)` with a pre-existing non-GitButler hook must
/// return `Skipped` and leave the original file untouched. This documents
/// the behavioral change from the old code which unconditionally overwrote.
#[test]
fn install_hook_without_force_preserves_existing_non_gb_hook() {
    let dir = TempDir::new().unwrap();
    let hooks_dir = dir.path().join("hooks");
    fs::create_dir(&hooks_dir).unwrap();

    let original_content = "#!/bin/sh\necho 'my precious hook'\n";
    fs::write(hooks_dir.join("pre-commit"), original_content).unwrap();

    let result = install_hook(&hooks_dir, ManagedHookType::PreCommit, false).unwrap();
    match result {
        HookInstallationResult::Skipped { hook_names } => {
            assert_eq!(hook_names, vec!["pre-commit"]);
        }
        other => panic!("expected Skipped, got {other:?}"),
    }

    // Original file must be untouched — no backup created, no content change.
    let content = fs::read_to_string(hooks_dir.join("pre-commit")).unwrap();
    assert_eq!(content, original_content, "original hook must be preserved");
    assert!(
        !hooks_dir.join("pre-commit-user").exists(),
        "no backup should be created without force"
    );
}
