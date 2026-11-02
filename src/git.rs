//! Hardened Git checkout discovery and identity validation.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::types::{GitCheckout, ResolutionError};

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
    // The returned common directory keeps its semantic form (any
    // `..` components intact). Canonical values are computed only
    // for owner comparison inside `main_worktree_root`, never for
    // the propagated path; if comparison cannot be established the
    // outcome is a typed failure, never a lexical retargeting.
    let git_common_dir = common_joined;
    // A submodule common directory is the sequence
    // <super>/.git/modules/<name>, never a bare "modules" component.
    let components: Vec<_> = git_common_dir
        .components()
        .map(|component| component.as_os_str().to_os_string())
        .collect();
    let is_submodule_layout = components
        .windows(2)
        .any(|pair| pair[0] == ".git" && pair[1] == "modules");
    if is_submodule_layout {
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
    // A real TOML parse (the declared exception): line scanning
    // misreads multiline-string contents as tables and keys.
    let Ok(table) = manifest.parse::<toml::Table>() else {
        return Err(ResolutionError::UndeterminedPackage(manifest_path));
    };
    let Some(package) = table.get("package") else {
        return Err(ResolutionError::VirtualWorkspace(manifest_path));
    };
    let found =
        package
            .get("name")
            .and_then(toml::Value::as_str)
            .filter(|name| {
                !name.is_empty()
                    && name.chars().all(|character| {
                        character.is_alphanumeric()
                            || matches!(character, '-' | '_' | '.')
                    })
            });
    match found {
        None => Err(ResolutionError::UndeterminedPackage(manifest_path)),
        Some(name) if name == expected => Ok(()),
        Some(name) => Err(ResolutionError::IdentityMismatch {
            expected: expected.to_string(),
            found: Some(name.to_string()),
        }),
    }
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
    let output = run_git(start, &["worktree", "list", "--porcelain", "-z"])?;
    // With -z every attribute line is NUL-terminated, so a pathname
    // containing a newline stays inside one segment; never split on
    // newlines here.
    let mut entries = Vec::new();
    for attribute in output.split(|byte| *byte == 0) {
        if let Some(path) = attribute.strip_prefix(b"worktree ".as_slice()) {
            entries.push(path_from_bytes(path)?);
        }
    }
    let entries = entries;
    // Git's separate-Git-directory listing names the Git directory's
    // parent as a worktree; only trust the owner match when the real
    // worktree is itself listed. Matching compares canonical values
    // only; the propagated paths are never rewritten.
    if entries.iter().any(|entry| entry == worktree_root) {
        let common_canonical =
            fs::canonicalize(git_common_dir).map_err(|error| {
                ResolutionError::GitFailed(format!(
                    "cannot establish common-directory comparison: {error}"
                ))
            })?;
        for entry in &entries {
            if let Ok(candidate) = fs::canonicalize(entry.join(".git"))
                && candidate == common_canonical
            {
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
///
/// Only the trailing newline framing is stripped; pathname bytes
/// (including trailing spaces and non-UTF8 names) pass through intact.
fn rev_parse(start: &Path, flag: &str) -> Result<PathBuf, ResolutionError> {
    let output = run_git_raw(start, &["rev-parse", flag])?;
    let mut bytes = output.as_slice();
    // Git frames output with exactly one trailing LF; a preceding CR
    // belongs to the pathname on POSIX and is preserved.
    if bytes.last() == Some(&b'\n') {
        bytes = &bytes[..bytes.len() - 1];
    }
    if bytes.is_empty() {
        return Err(ResolutionError::GitFailed(format!(
            "empty output for {flag}"
        )));
    }
    path_from_bytes(bytes)
}

/// Builds a path from raw Git output bytes without lossy conversion.
///
/// # Errors
///
/// Returns [`ResolutionError::GitFailed`] for non-UTF8 bytes on
/// platforms without byte-native paths.
fn path_from_bytes(bytes: &[u8]) -> Result<PathBuf, ResolutionError> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(PathBuf::from(std::ffi::OsString::from_vec(bytes.to_vec())))
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(bytes.to_vec())
            .map(PathBuf::from)
            .map_err(|_| {
                ResolutionError::GitFailed(
                    "non-UTF8 pathname from git".to_string(),
                )
            })
    }
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
    // Attribute spawn failures before spawning: a missing start
    // directory also surfaces as ErrorKind::NotFound from the spawn
    // itself, so it must not be read as a missing executable.
    if let Err(error) = std::fs::metadata(start) {
        return Err(ResolutionError::GitFailed(format!(
            "start directory {}: {error}",
            start.display()
        )));
    }
    let output = command.output().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ResolutionError::GitUnavailable
        } else {
            ResolutionError::GitFailed(format!("spawn git: {error}"))
        }
    })?;
    Ok((output.status.success(), output.stdout, output.stderr))
}
