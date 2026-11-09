//! Core types shared by every resolution path.

use std::{
    fmt::{Display, Formatter},
    path::{Component, Path, PathBuf},
};

/// One conventional directory class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectoryClass {
    /// Configuration files.
    Configuration,
    /// Mutable application state.
    State,
    /// Disposable cache data.
    Cache,
    /// Durable application data.
    Data,
}

impl DirectoryClass {
    /// Returns the conventional path segment for this class.
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

/// Which genus of discoverer occupies a resolver slot.
///
/// A resolver holds at most one discoverer per genus, so the genus
/// unambiguously addresses per-discoverer argument routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiscovererGenus {
    /// Version-control discoverers (for example Git, Mercurial).
    Vcs,
    /// Development-marker discoverers (for example Cargo, Pyproject).
    Devfile,
}

impl DiscovererGenus {
    /// Returns the stable label naming this genus in diagnostics.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Vcs => "Vcs",
            Self::Devfile => "Devfile",
        }
    }
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

/// A family of per-kind caller-supplied roots with gaps.
///
/// Each `Some` value is a complete directory for its class, used
/// literally after shape validation; each `None` is a gap the library
/// fills through the normal chain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartialRoots {
    /// Complete Configuration-class directory, when supplied.
    pub configuration: Option<PathBuf>,
    /// Complete State-class directory, when supplied.
    pub state: Option<PathBuf>,
    /// Complete Cache-class directory, when supplied.
    pub cache: Option<PathBuf>,
    /// Complete Data-class directory, when supplied.
    pub data: Option<PathBuf>,
}

/// A fully resolved family: one root per class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedRoots {
    /// The resolved Configuration-class root.
    pub configuration: ResolvedRoot,
    /// The resolved State-class root.
    pub state: ResolvedRoot,
    /// The resolved Cache-class root.
    pub cache: ResolvedRoot,
    /// The resolved Data-class root.
    pub data: ResolvedRoot,
}

/// Git checkout topology. A private observation inside the Git
/// discoverer; the public discovery output is a single selected base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GitCheckout {
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
    /// A `[package]` table exists but its name is undeterminable.
    UndeterminedPackage(PathBuf),
    /// The platform offers no root for the requested class.
    Unavailable(DirectoryClass),
    /// A layer or manifest could not be read.
    Unreadable(PathBuf),
    /// Two candidates canonicalize to the same target.
    Ambiguous(PathBuf),
    /// A discoverer argument was unknown, or its value invalid.
    InvalidDiscovererArgument {
        /// The discoverer (or genus label) that rejected the argument.
        discoverer: String,
        /// The offending argument key.
        key: String,
        /// The offending argument value.
        value: String,
    },
    /// A second discoverer was registered for an occupied genus.
    DuplicateDiscoverer(DiscovererGenus),
}

/// Refuses Windows drive-relative and root-relative shapes.
///
/// These consult ambient drive state or replace the base during a
/// join, so they are rejected before any join happens — including
/// the explicit tier's permitted ordinary-relative join. This is a
/// no-op on other platforms, where such spellings are ordinary
/// relative paths. Absolute, non-empty paths pass through unchanged;
/// nothing is canonicalized or otherwise rewritten.
pub(crate) fn refuse_windows_ambient_shape(
    path: &Path,
) -> Result<(), ResolutionError> {
    #[cfg(windows)]
    {
        use std::path::Component;
        let mut components = path.components();
        match components.next() {
            // Drive-relative (C:foo) consults ambient drive state.
            Some(Component::Prefix(_)) => {
                if !matches!(components.next(), Some(Component::RootDir)) {
                    return Err(ResolutionError::InvalidName(
                        path.display().to_string(),
                    ));
                }
            },
            // Root-relative (\foo) replaces the base during a join.
            Some(Component::RootDir) => {
                return Err(ResolutionError::InvalidName(
                    path.display().to_string(),
                ));
            },
            _ => {},
        }
    }
    let _ = path;
    Ok(())
}

/// Refuses empty and relative root shapes.
///
/// Windows ambient-dependent shapes are refused first: on Windows
/// they are not absolute, so an absoluteness check alone would
/// misreport them as ordinary relative paths.
pub(crate) fn validate_root_shape(path: &Path) -> Result<(), ResolutionError> {
    if path.as_os_str().is_empty() {
        return Err(ResolutionError::EmptyPath);
    }
    refuse_windows_ambient_shape(path)?;
    if !path.is_absolute() {
        return Err(ResolutionError::RelativeStartDirectory(
            path.to_path_buf(),
        ));
    }
    Ok(())
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
            Self::UndeterminedPackage(path) => write!(
                formatter,
                "{} has a package table with an undeterminable name",
                path.display()
            ),
            Self::Unavailable(class) => {
                write!(formatter, "no platform root for {class:?}")
            },
            Self::InvalidDiscovererArgument {
                discoverer,
                key,
                value,
            } => write!(
                formatter,
                "invalid argument {key:?}={value:?} for discoverer {discoverer}"
            ),
            Self::DuplicateDiscoverer(genus) => write!(
                formatter,
                "duplicate discoverer for genus {}",
                genus.label()
            ),
            Self::Unreadable(path) => {
                write!(formatter, "cannot read {}", path.display())
            },
            Self::Ambiguous(path) => {
                write!(
                    formatter,
                    "ambiguous candidates collapse to {}",
                    path.display()
                )
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
