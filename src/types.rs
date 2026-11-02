//! Core types shared by every resolution path.

use std::{
    fmt::{Display, Formatter},
    path::{Component, Path, PathBuf},
};

/// One conventional directory kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectoryKind {
    /// Configuration files.
    Configuration,
    /// Mutable application state.
    State,
    /// Disposable cache data.
    Cache,
    /// Durable application data.
    Data,
}

impl DirectoryKind {
    /// Returns the conventional path segment for this kind.
    #[must_use]
    pub fn segment(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::State => "state",
            Self::Cache => "cache",
            Self::Data => "data",
        }
    }
}

/// Which checkout a development root belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevelopmentScope {
    /// Rooted at the common worktree, shared by linked worktrees.
    Common,
    /// Rooted at the current worktree.
    Worktree,
}

/// Where a resolved root came from. This set is closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provenance {
    /// Supplied explicitly by the caller (flag or environment).
    CallerProvided,
    /// Beneath a caller-supplied directory.
    SuppliedDirectory,
    /// Beneath the common worktree's development tree.
    DevelopmentCommon,
    /// Beneath the current worktree's development tree.
    DevelopmentWorktree,
    /// Platform root influenced by a platform environment variable.
    PlatformEnvironment,
    /// Computed platform default.
    PlatformDefault,
}

/// One resolved root together with its provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedRoot {
    /// The candidate path. It may not exist; resolution creates nothing.
    pub path: PathBuf,
    /// The tier that supplied `path`.
    pub provenance: Provenance,
}

/// Git checkout topology. The three roots are distinct on purpose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitCheckout {
    /// Root of the current worktree.
    pub worktree_root: PathBuf,
    /// Root of the worktree owning the Git common directory.
    pub common_worktree_root: PathBuf,
    /// The Git common directory itself.
    pub git_common_dir: PathBuf,
}

/// Failure modes of resolution. Absence is not represented here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolutionError {
    /// A supplied path was empty.
    EmptyPath,
    /// A start directory was not absolute.
    RelativeStartDirectory(PathBuf),
    /// A name failed the token grammar.
    InvalidName(String),
    /// The `git` executable could not be run.
    GitUnavailable,
    /// A Git command failed for a reason other than absence of a repo.
    GitFailed(String),
    /// The start directory is not inside a Git repository.
    NotRepository(PathBuf),
    /// A Git layout this crate does not model.
    UnsupportedTopology(String),
    /// A Cargo manifest did not carry the expected package name.
    IdentityMismatch {
        /// The package name the caller required.
        expected: String,
        /// The package name found, when any.
        found: Option<String>,
    },
    /// The checkout root is a virtual Cargo workspace.
    VirtualWorkspace(PathBuf),
    /// The platform offers no root for the requested kind.
    Unavailable(DirectoryKind),
    /// A layer or manifest could not be read.
    Unreadable(PathBuf),
}

impl Display for ResolutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => write!(formatter, "supplied path is empty"),
            Self::RelativeStartDirectory(path) => write!(
                formatter,
                "start directory {} is not absolute",
                path.display()
            ),
            Self::InvalidName(name) => {
                write!(formatter, "invalid name {name:?}")
            },
            Self::GitUnavailable => {
                write!(formatter, "git executable unavailable")
            },
            Self::GitFailed(message) => {
                write!(formatter, "git command failed: {message}")
            },
            Self::NotRepository(path) => write!(
                formatter,
                "{} is not inside a git repository",
                path.display()
            ),
            Self::UnsupportedTopology(message) => {
                write!(formatter, "unsupported git topology: {message}")
            },
            Self::IdentityMismatch { expected, found } => write!(
                formatter,
                "expected package {expected:?}, found {found:?}"
            ),
            Self::VirtualWorkspace(path) => write!(
                formatter,
                "{} is a virtual cargo workspace",
                path.display()
            ),
            Self::Unavailable(kind) => {
                write!(formatter, "no platform root for {kind:?}")
            },
            Self::Unreadable(path) => {
                write!(formatter, "cannot read {}", path.display())
            },
        }
    }
}

impl std::error::Error for ResolutionError {}

/// Validates one joined path segment against the token grammar.
///
/// # Errors
///
/// Returns [`ResolutionError::InvalidName`] for empty names, names with
/// separators, Windows prefixes or alternate-data-stream colons, NUL
/// bytes, and the traversal-only `.` / `..` names.
pub fn validate_token(name: &str) -> Result<(), ResolutionError> {
    let invalid = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', ':', '\0']);
    if invalid {
        return Err(ResolutionError::InvalidName(name.to_string()));
    }
    Ok(())
}

/// Validates a layout override as relative components beneath a root.
///
/// # Errors
///
/// Returns [`ResolutionError::InvalidName`] for rooted, prefixed, or
/// traversal components and for an empty override.
pub fn validate_relative_components(
    path: &Path,
) -> Result<(), ResolutionError> {
    if path.as_os_str().is_empty() {
        return Err(ResolutionError::EmptyPath);
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {},
            _ => {
                return Err(ResolutionError::InvalidName(
                    path.display().to_string(),
                ));
            },
        }
    }
    Ok(())
}

/// Normalizes a path lexically without touching the filesystem.
#[must_use]
pub fn normalize_lexical(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            },
            Component::CurDir => {},
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}
