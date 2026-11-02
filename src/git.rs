//! Hardened Git checkout discovery and identity validation.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::types::{GitCheckout, ResolutionError, normalize_lexical};

/// Discovers checkout topology from an explicit start directory.
///
/// Relative `--git-common-dir` output resolves against `start`, the exact
/// working directory the Git subprocess runs in. The common worktree
/// root comes from `git worktree list`, never from assuming the common
/// directory's parent is its owner.
///
/// # Errors
///
/// Returns [`ResolutionError::GitUnavailable`] when Git cannot run,
/// [`ResolutionError::NotRepository`] outside a repository,
/// [`ResolutionError::GitFailed`] for other Git failures, and
/// [`ResolutionError::UnsupportedTopology`] for bare repositories,
/// submodule layouts, and other unmodelled topologies.
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
    if git_common_dir
        .components()
        .any(|component| component.as_os_str() == "modules")
    {
        return Err(ResolutionError::UnsupportedTopology(format!(
            "submodule common directory {}",
            git_common_dir.display()
        )));
    }
    let common_worktree_root =
        main_worktree_root(start, &worktree_root, &git_common_dir)?;
    Ok(GitCheckout {
        worktree_root,
        common_worktree_root,
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
///
/// The key must be exactly `name` (so `name.workspace` inheritance does
/// not masquerade as a name), the value must be a quoted string, and a
/// parsed value that is not a plausible package name is treated as
/// unknown rather than reported as a found name.
fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() != "name" {
            continue;
        }
        let value = value.trim();
        let quote = value.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let parsed = value[1..].split(quote).next()?.to_string();
        let plausible = !parsed.is_empty()
            && parsed
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'));
        return plausible.then_some(parsed);
    }
    None
}

/// Returns the worktree owning the Git common directory.
///
/// The owner is the listed worktree whose `.git` directory is the
/// common directory. `git worktree list` alone is not sufficient: for
/// a separate Git directory it names the Git directory's parent, so a
/// single-worktree checkout falls back to the current worktree root.
fn main_worktree_root(
    start: &Path,
    worktree_root: &Path,
    git_common_dir: &Path,
) -> Result<PathBuf, ResolutionError> {
    let output = run_git(start, &["worktree", "list", "--porcelain"])?;
    let text = String::from_utf8(output)
        .map_err(|_| ResolutionError::GitFailed("non-UTF8 output".into()))?;
    let entries: Vec<PathBuf> = text
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(|path| PathBuf::from(path.trim()))
        .collect();
    // Git's separate-Git-directory listing names the Git directory's
    // parent as a worktree; only trust the owner match when the real
    // worktree is itself listed.
    if entries.iter().any(|entry| entry == worktree_root) {
        for entry in &entries {
            if entry.join(".git") == git_common_dir {
                return Ok(entry.clone());
            }
        }
    }
    if entries.len() == 1 {
        return Ok(worktree_root.to_path_buf());
    }
    Err(ResolutionError::UnsupportedTopology(format!(
        "no listed worktree owns common directory {}",
        git_common_dir.display()
    )))
}

/// Runs one `git rev-parse` query, classifying failures honestly.
fn rev_parse(start: &Path, flag: &str) -> Result<PathBuf, ResolutionError> {
    let output = run_git_raw(start, &["rev-parse", flag])?;
    let text = String::from_utf8(output)
        .map_err(|_| ResolutionError::GitFailed("non-UTF8 output".into()))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ResolutionError::GitFailed(format!(
            "empty output for {flag}"
        )));
    }
    Ok(PathBuf::from(trimmed))
}

/// Runs a Git command that must succeed, returning stdout bytes.
fn run_git(start: &Path, args: &[&str]) -> Result<Vec<u8>, ResolutionError> {
    let (status_success, stdout, stderr) = run_git_status(start, args)?;
    if status_success {
        return Ok(stdout);
    }
    Err(classify_failure(start, &stderr))
}

/// Runs a Git command, mapping spawn failure to `GitUnavailable`.
fn run_git_raw(
    start: &Path,
    args: &[&str],
) -> Result<Vec<u8>, ResolutionError> {
    let (status_success, stdout, stderr) = run_git_status(start, args)?;
    if status_success {
        return Ok(stdout);
    }
    Err(classify_failure(start, &stderr))
}

/// Separates "not a repository" from every other Git failure.
///
/// A directory Git recognizes (`--git-dir` succeeds) whose worktree
/// query fails is bare (unsupported topology) or broken (a surfaced
/// failure), never a silent "not a repository".
fn classify_failure(start: &Path, stderr: &[u8]) -> ResolutionError {
    let message = String::from_utf8_lossy(stderr).trim().to_string();
    if run_git_status(start, &["rev-parse", "--git-dir"])
        .is_ok_and(|(success, _, _)| success)
    {
        let bare =
            run_git_status(start, &["rev-parse", "--is-bare-repository"])
                .ok()
                .filter(|(success, _, _)| *success)
                .map(|(_, stdout, _)| {
                    String::from_utf8_lossy(&stdout).trim() == "true"
                })
                .unwrap_or(false);
        if bare {
            return ResolutionError::UnsupportedTopology(format!(
                "bare repository at {}",
                start.display()
            ));
        }
        return ResolutionError::GitFailed(message);
    }
    if message.contains("not a git repository") {
        return ResolutionError::NotRepository(start.to_path_buf());
    }
    ResolutionError::GitFailed(message)
}

/// Spawns Git with a scrubbed environment.
fn run_git_status(
    start: &Path,
    args: &[&str],
) -> Result<(bool, Vec<u8>, Vec<u8>), ResolutionError> {
    let mut command = Command::new("git");
    command.args(args).current_dir(start);
    // Blast-by-prefix catches future GIT_* redirects by default.
    // Revisit selective retention only when a consumer must pass one
    // specific GIT_* variable through deliberately.
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(&key);
        }
    }
    // Set after the blast: env-then-remove would unset it again.
    command.env("GIT_CONFIG_NOSYSTEM", "1");
    let output = command
        .output()
        .map_err(|_| ResolutionError::GitUnavailable)?;
    Ok((output.status.success(), output.stdout, output.stderr))
}
