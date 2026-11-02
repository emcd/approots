//! Hardened Git checkout discovery and identity validation.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::types::{GitCheckout, ResolutionError, normalize_lexical};

/// Discovers checkout topology from an explicit start directory.
///
/// Relative `--git-common-dir` output resolves against `start`, the exact
/// working directory the Git subprocess runs in.
///
/// # Errors
///
/// Returns [`ResolutionError::GitUnavailable`] when Git cannot run,
/// [`ResolutionError::NotRepository`] outside a repository, and
/// [`ResolutionError::UnsupportedTopology`] for layouts whose common
/// directory is not a `.git` directory (bare repositories, separate Git
/// directories, and submodule layouts included).
pub fn discover_checkout(
    start: &Path,
) -> Result<GitCheckout, ResolutionError> {
    if !start.is_absolute() {
        return Err(ResolutionError::RelativeStartDirectory(
            start.to_path_buf(),
        ));
    }
    let worktree_root = rev_parse(start, "--show-toplevel")?;
    let common_raw = rev_parse(start, "--git-common-dir")?;
    let common_joined = if common_raw.is_relative() {
        start.join(&common_raw)
    } else {
        common_raw
    };
    let git_common_dir = normalize_lexical(&common_joined);
    let is_dot_git = git_common_dir
        .file_name()
        .is_some_and(|name| name == ".git");
    if !is_dot_git {
        return Err(ResolutionError::UnsupportedTopology(format!(
            "common directory {} is not a .git directory",
            git_common_dir.display()
        )));
    }
    let Some(common_worktree_root) = git_common_dir.parent() else {
        return Err(ResolutionError::UnsupportedTopology(format!(
            "common directory {} has no parent",
            git_common_dir.display()
        )));
    };
    Ok(GitCheckout {
        worktree_root,
        common_worktree_root: common_worktree_root.to_path_buf(),
        git_common_dir,
    })
}

/// Validates that a checkout carries an expected Cargo package name.
///
/// Only the `Cargo.toml` at the worktree root is read; the search never
/// leaves the checkout.
///
/// # Errors
///
/// Returns [`ResolutionError::VirtualWorkspace`] when the root manifest
/// has no `[package]` table, [`ResolutionError::IdentityMismatch`] when
/// the name differs, and [`ResolutionError::Unreadable`] when the
/// manifest cannot be read.
pub fn validate_cargo_package(
    checkout: &GitCheckout,
    expected: &str,
) -> Result<(), ResolutionError> {
    let manifest_path = checkout.worktree_root.join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|_| ResolutionError::Unreadable(manifest_path.clone()))?;
    let found = package_name(&manifest);
    match found {
        None => Err(ResolutionError::VirtualWorkspace(manifest_path)),
        Some(name) if name == expected => Ok(()),
        Some(name) => Err(ResolutionError::IdentityMismatch {
            expected: expected.to_string(),
            found: Some(name),
        }),
    }
}

/// Extracts `[package].name` without a TOML dependency.
fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if in_package && trimmed.starts_with("name") {
            let value = trimmed.split('=').nth(1)?.trim();
            return value
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
                .into();
        }
    }
    None
}

/// Runs one `git rev-parse` query with a scrubbed environment.
fn rev_parse(start: &Path, flag: &str) -> Result<PathBuf, ResolutionError> {
    let mut command = Command::new("git");
    command
        .args(["rev-parse", flag])
        .current_dir(start)
        .env("GIT_CONFIG_NOSYSTEM", "1");
    // Blast-by-prefix catches future GIT_* redirects by default.
    // Revisit selective retention only when a consumer must pass one
    // specific GIT_* variable through deliberately.
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(&key);
        }
    }
    let output = command
        .output()
        .map_err(|_| ResolutionError::GitUnavailable)?;
    if !output.status.success() {
        return Err(ResolutionError::NotRepository(start.to_path_buf()));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|_| ResolutionError::GitFailed("non-UTF8 output".into()))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ResolutionError::GitFailed(format!(
            "empty output for {flag}"
        )));
    }
    Ok(PathBuf::from(trimmed))
}
