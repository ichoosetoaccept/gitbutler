mod detect_hook_manager;
mod detect_hook_manager_in_hooks_dir;

use std::fs;

use tempfile::TempDir;

pub(super) use crate::common::create_hooks_dir as create_hooks_dir_result;
pub(super) const TEST_ENV_VAR: &str = "_GITBUTLER_TEST_BINARY_AVAILABLE";

/// Create a temp directory with a `hooks/` subdirectory, returning both.
/// Panics on I/O failure — suitable for test functions that don't return `Result`.
pub(super) fn create_hooks_dir() -> (TempDir, std::path::PathBuf) {
    create_hooks_dir_result().unwrap()
}

/// Write a minimal `prek.toml` config file in the given project directory.
pub(super) fn create_prek_config(project_dir: &std::path::Path) {
    fs::write(project_dir.join("prek.toml"), "# prek config").unwrap();
}

/// Run a closure with the test binary-availability flag set (simulates prek being in PATH).
pub(super) fn with_binary_available<F: FnOnce() -> R, R>(f: F) -> R {
    temp_env::with_var(TEST_ENV_VAR, Some("1"), f)
}

/// Run a closure with the test binary-availability flag set to "0" (simulates prek NOT in PATH).
pub(super) fn with_binary_unavailable<F: FnOnce() -> R, R>(f: F) -> R {
    temp_env::with_var(TEST_ENV_VAR, Some("0"), f)
}
