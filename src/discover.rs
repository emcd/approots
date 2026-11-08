//! Discoverers that produce bases for local-structure resolution.
//!
//! Version-control discoverers select a checkout base; devfile
//! discoverers validate a marker at a selected base or search upward
//! for one on their own. A resolver holds at most one discoverer per
//! [`DiscovererGenus`], so the genus unambiguously addresses argument
//! routing. Arguments are validated and deserialized before any
//! discoverer executes.

use std::{
    collections::BTreeMap,
    fmt::Debug,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use crate::types::{DiscovererGenus, Provenance, ResolutionError};

/// A single discovered base together with its provenance.
///
/// Git scope `Common` selects the common worktree root with
/// development-common provenance; scope `Local` (and Cargo-only
/// discovery) selects the worktree root with development-worktree
/// provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredBase {
    /// The selected base directory.
    pub path: PathBuf,
    /// The provenance the structured root inherits.
    pub provenance: Provenance,
}

/// The common behavior of every discoverer.
pub trait Discoverer: Debug + Send + Sync {
    /// Returns the genus slot this discoverer occupies.
    fn genus(&self) -> DiscovererGenus;
}

/// A version-control discoverer selecting a checkout base.
pub trait VcsDiscoverer: Discoverer {
    /// Validates an argument bag without running any discovery.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError::InvalidDiscovererArgument`] for
    /// unknown keys or invalid values.
    fn validate_args(
        &self,
        args: &BTreeMap<String, String>,
    ) -> Result<(), ResolutionError>;

    /// Selects a base at an explicit start directory.
    ///
    /// Genuine absence of a repository is `Ok(None)`; execution
    /// failures and unsupported topologies are errors.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError`] for invalid arguments, relative
    /// starts, execution failures, and unsupported topologies.
    fn discover(
        &self,
        start: &Path,
        args: &BTreeMap<String, String>,
    ) -> Result<Option<DiscoveredBase>, ResolutionError>;

    /// Clones this discoverer behind its trait object.
    fn clone_box(&self) -> Box<dyn VcsDiscoverer>;
}

/// A development-marker discoverer validating or finding a base.
pub trait DevfileDiscoverer: Discoverer {
    /// Validates an argument bag without running any discovery.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError::InvalidDiscovererArgument`] for
    /// unknown keys or invalid values.
    fn validate_args(
        &self,
        args: &BTreeMap<String, String>,
    ) -> Result<(), ResolutionError>;

    /// Validates the marker at an already-selected VCS base.
    ///
    /// Only the marker at `base` is read: no upward search beyond the
    /// base and no descent into nested member manifests.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a missing, unreadable, malformed,
    /// virtual-workspace, or mismatched marker — never a fallthrough.
    fn validate_at_base(
        &self,
        base: &Path,
        args: &BTreeMap<String, String>,
        application: &str,
    ) -> Result<(), ResolutionError>;

    /// Searches upward for a marker without a VCS base.
    ///
    /// The nearest marker wins; finding none means the discovered tier
    /// is absent rather than an error.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError`] for invalid arguments, relative
    /// starts, unreadable starts or markers, and identity mismatches.
    fn discover_standalone(
        &self,
        start: &Path,
        args: &BTreeMap<String, String>,
        application: &str,
    ) -> Result<Option<DiscoveredBase>, ResolutionError>;

    /// Clones this discoverer behind its trait object.
    fn clone_box(&self) -> Box<dyn DevfileDiscoverer>;
}

/// Which checkout root the Git discoverer selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The common worktree root shared by linked worktrees.
    Common,
    /// The current worktree root.
    Local,
}

impl Scope {
    /// Parses a scope argument value.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "common" => Some(Self::Common),
            "local" => Some(Self::Local),
            _ => None,
        }
    }
}

/// Git checkout discovery behind the discoverer interface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VcsDiscovererGit;

impl VcsDiscovererGit {
    /// Argument key selecting the anchor root.
    pub const SCOPE_KEY: &'static str = "scope";
    /// `scope` value selecting the common worktree root.
    pub const SCOPE_COMMON: &'static str = "common";
    /// `scope` value selecting the current worktree root.
    pub const SCOPE_LOCAL: &'static str = "local";

    fn scope_of(
        args: &BTreeMap<String, String>,
    ) -> Result<Scope, ResolutionError> {
        match args.get(Self::SCOPE_KEY).map(String::as_str) {
            None => Ok(Scope::Local),
            Some(value) => Scope::parse(value).ok_or_else(|| {
                ResolutionError::InvalidDiscovererArgument {
                    discoverer: "VcsDiscovererGit".to_string(),
                    key: Self::SCOPE_KEY.to_string(),
                    value: value.to_string(),
                }
            }),
        }
    }
}

impl Discoverer for VcsDiscovererGit {
    fn genus(&self) -> DiscovererGenus {
        DiscovererGenus::Vcs
    }
}

impl VcsDiscoverer for VcsDiscovererGit {
    fn validate_args(
        &self,
        args: &BTreeMap<String, String>,
    ) -> Result<(), ResolutionError> {
        for (key, value) in args {
            if key != Self::SCOPE_KEY {
                return Err(ResolutionError::InvalidDiscovererArgument {
                    discoverer: "VcsDiscovererGit".to_string(),
                    key: key.clone(),
                    value: value.clone(),
                });
            }
        }
        // Deserialize eagerly so invalid values fail before any
        // subprocess runs.
        Self::scope_of(args).map(|_| ())
    }

    fn discover(
        &self,
        start: &Path,
        args: &BTreeMap<String, String>,
    ) -> Result<Option<DiscoveredBase>, ResolutionError> {
        let scope = Self::scope_of(args)?;
        if !start.is_absolute() {
            return Err(ResolutionError::RelativeStartDirectory(
                start.to_path_buf(),
            ));
        }
        match crate::git::discover_checkout(start) {
            Ok(checkout) => {
                let (path, provenance) = match scope {
                    Scope::Common => (
                        checkout.common_worktree_root,
                        Provenance::DevelopmentCommon,
                    ),
                    Scope::Local => (
                        checkout.worktree_root,
                        Provenance::DevelopmentWorktree,
                    ),
                };
                Ok(Some(DiscoveredBase { path, provenance }))
            },
            // Genuine absence ends discovery; the caller falls
            // through by precedence without a standalone search.
            Err(ResolutionError::NotRepository(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn clone_box(&self) -> Box<dyn VcsDiscoverer> {
        Box::new(*self)
    }
}

/// Cargo marker discovery and identity validation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DevfileDiscovererCargo;

impl DevfileDiscovererCargo {
    /// Argument key overriding the expected package name.
    pub const EXPECTED_NAME_KEY: &'static str = "expected_name";

    fn expected_identity<'a>(
        args: &'a BTreeMap<String, String>,
        application: &'a str,
    ) -> &'a str {
        args.get(Self::EXPECTED_NAME_KEY)
            .map(String::as_str)
            .unwrap_or(application)
    }
}

impl Discoverer for DevfileDiscovererCargo {
    fn genus(&self) -> DiscovererGenus {
        DiscovererGenus::Devfile
    }
}

impl DevfileDiscoverer for DevfileDiscovererCargo {
    fn validate_args(
        &self,
        args: &BTreeMap<String, String>,
    ) -> Result<(), ResolutionError> {
        for (key, value) in args {
            if key != Self::EXPECTED_NAME_KEY {
                return Err(ResolutionError::InvalidDiscovererArgument {
                    discoverer: "DevfileDiscovererCargo".to_string(),
                    key: key.clone(),
                    value: value.clone(),
                });
            }
            if value.is_empty() {
                return Err(ResolutionError::InvalidDiscovererArgument {
                    discoverer: "DevfileDiscovererCargo".to_string(),
                    key: key.clone(),
                    value: value.clone(),
                });
            }
        }
        Ok(())
    }

    fn validate_at_base(
        &self,
        base: &Path,
        args: &BTreeMap<String, String>,
        application: &str,
    ) -> Result<(), ResolutionError> {
        let manifest_path = base.join("Cargo.toml");
        let content = fs::read_to_string(&manifest_path)
            .map_err(|_| ResolutionError::Unreadable(manifest_path.clone()))?;
        let found = cargo_package_name(&manifest_path, &content)?;
        let expected = Self::expected_identity(args, application);
        if found == expected {
            Ok(())
        } else {
            Err(ResolutionError::IdentityMismatch {
                expected: expected.to_string(),
                found: Some(found),
            })
        }
    }

    fn discover_standalone(
        &self,
        start: &Path,
        args: &BTreeMap<String, String>,
        application: &str,
    ) -> Result<Option<DiscoveredBase>, ResolutionError> {
        if !start.is_absolute() {
            return Err(ResolutionError::RelativeStartDirectory(
                start.to_path_buf(),
            ));
        }
        // A missing start is a typed error, not a silent search from
        // wherever the nearest readable ancestor happens to be.
        if fs::metadata(start).is_err() {
            return Err(ResolutionError::Unreadable(start.to_path_buf()));
        }
        let mut directory = start.to_path_buf();
        // The physical form drives ancestry decisions; the semantic
        // form is what markers are read through and what is returned.
        let mut physical = fs::canonicalize(&directory)
            .map_err(|_| ResolutionError::Unreadable(directory.clone()))?;
        loop {
            let manifest_path = directory.join("Cargo.toml");
            match fs::read_to_string(&manifest_path) {
                Ok(content) => {
                    let found = cargo_package_name(&manifest_path, &content)?;
                    let expected = Self::expected_identity(args, application);
                    if found != expected {
                        return Err(ResolutionError::IdentityMismatch {
                            expected: expected.to_string(),
                            found: Some(found),
                        });
                    }
                    return Ok(Some(DiscoveredBase {
                        path: directory,
                        provenance: Provenance::DevelopmentWorktree,
                    }));
                },
                // No marker here: keep walking upward. Permission and
                // other IO failures are faults, never "no marker".
                Err(error) if error.kind() == ErrorKind::NotFound => {},
                Err(_) => {
                    return Err(ResolutionError::Unreadable(manifest_path));
                },
            }
            // Step to the physical parent. A lexical pop is preferred
            // when it provably names the same directory — keeping
            // ordinary paths clean — and `..` is joined otherwise, so
            // symlinked or dot-dot segments never misroute the walk.
            // Either way the returned base keeps its semantic,
            // non-canonicalized form.
            let Some(physical_parent) = physical.parent() else {
                return Ok(None);
            };
            let physical_parent = physical_parent.to_path_buf();
            let lexical_parent = directory.parent().map(Path::to_path_buf);
            let lexical_names_parent =
                lexical_parent.as_ref().is_some_and(|lexical| {
                    fs::canonicalize(lexical)
                        .is_ok_and(|candidate| candidate == physical_parent)
                });
            match lexical_parent {
                Some(lexical) if lexical_names_parent => {
                    directory = lexical;
                },
                _ => {
                    directory = directory.join("..");
                },
            }
            physical = physical_parent;
        }
    }

    fn clone_box(&self) -> Box<dyn DevfileDiscoverer> {
        Box::new(*self)
    }
}

/// Reads the package name out of a Cargo manifest.
///
/// A real TOML parse (the declared exception): line scanning misreads
/// multiline-string contents as tables and keys.
fn cargo_package_name(
    manifest_path: &Path,
    content: &str,
) -> Result<String, ResolutionError> {
    let Ok(table) = content.parse::<toml::Table>() else {
        return Err(ResolutionError::UndeterminedPackage(
            manifest_path.to_path_buf(),
        ));
    };
    let Some(package) = table.get("package") else {
        return Err(ResolutionError::VirtualWorkspace(
            manifest_path.to_path_buf(),
        ));
    };
    package
        .get("name")
        .and_then(toml::Value::as_str)
        .filter(|name| {
            !name.is_empty()
                && name.chars().all(|character| {
                    character.is_alphanumeric()
                        || matches!(character, '-' | '_' | '.')
                })
        })
        .map(str::to_string)
        .ok_or_else(|| {
            ResolutionError::UndeterminedPackage(manifest_path.to_path_buf())
        })
}
