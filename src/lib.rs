//! Directory locations for application caches, configuration, and state.
//!
//! The crate resolves candidate paths only. It never creates, modifies,
//! deletes, or locks anything, never parses or merges consumer
//! configuration, and never enforces ownership or permissions.
//! `Cargo.toml` identity parsing is the sole declared exception.

pub mod discover;
pub mod layers;
pub mod local;
pub mod platform;
pub mod types;

mod git;

pub use discover::{
    DevfileDiscoverer, DevfileDiscovererCargo, DiscoveredBase, Discoverer,
    Scope, VcsDiscoverer, VcsDiscovererGit,
};
pub use layers::LayeredRoots;
pub use local::{LocalStructure, local_root};
pub use platform::{platform_root, sanitize_name};
pub use types::{
    DirectoryClass, DiscovererGenus, Provenance, ResolutionError,
    ResolvedRoot, validate_token,
};

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Resolves one class beneath an explicitly supplied directory.
///
/// # Errors
///
/// Returns [`ResolutionError::RelativeStartDirectory`] when `base` is
/// not absolute and [`ResolutionError::EmptyPath`] when it is empty.
pub fn supplied_root(
    base: &Path,
    class: DirectoryClass,
) -> Result<ResolvedRoot, ResolutionError> {
    if base.as_os_str().is_empty() {
        return Err(ResolutionError::EmptyPath);
    }
    if !base.is_absolute() {
        return Err(ResolutionError::RelativeStartDirectory(
            base.to_path_buf(),
        ));
    }
    Ok(ResolvedRoot {
        path: base.join(class.segment()),
        provenance: Provenance::SuppliedDirectory,
    })
}

/// Refuses Windows drive-relative and root-relative root shapes.
///
/// On Windows these consult ambient drive state or replace the base
/// during a join, so they are rejected before any join happens. On
/// other platforms such prefixes cannot occur.
fn validate_root_shape(path: &Path) -> Result<(), ResolutionError> {
    #[cfg(windows)]
    {
        use std::path::Component;
        let mut components = path.components();
        match components.next() {
            // Drive-relative (C:foo) has a prefix but no root.
            Some(Component::Prefix(_)) => {
                if !matches!(components.next(), Some(Component::RootDir)) {
                    return Err(ResolutionError::InvalidName(
                        path.display().to_string(),
                    ));
                }
            },
            // Root-relative (\foo) consults the ambient drive.
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

/// Caller-driven resolver bound to one application name.
///
/// Discovery is opt-in: only registered discoverers ever run, and only
/// for resolutions the explicit-root tier has not already decided.
#[derive(Debug)]
pub struct Resolver {
    application: String,
    structure: LocalStructure,
    distinct_directory_classes: bool,
    vcs: Option<Box<dyn VcsDiscoverer>>,
    devfile: Option<Box<dyn DevfileDiscoverer>>,
    arguments: BTreeMap<DiscovererGenus, BTreeMap<String, String>>,
}

impl Clone for Resolver {
    fn clone(&self) -> Self {
        Self {
            application: self.application.clone(),
            structure: self.structure.clone(),
            distinct_directory_classes: self.distinct_directory_classes,
            vcs: self.vcs.as_ref().map(|vcs| vcs.clone_box()),
            devfile: self.devfile.as_ref().map(|dev| dev.clone_box()),
            arguments: self.arguments.clone(),
        }
    }
}

impl Resolver {
    /// Creates a resolver for `application`, with no discoverers.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError::InvalidName`] when the application
    /// name fails the token grammar.
    pub fn new(
        application: impl Into<String>,
    ) -> Result<Self, ResolutionError> {
        let application = application.into();
        validate_token(&application)?;
        Ok(Self {
            application,
            structure: LocalStructure::default(),
            distinct_directory_classes: false,
            vcs: None,
            devfile: None,
            arguments: BTreeMap::new(),
        })
    }

    /// Replaces the local structure mated onto discovered bases.
    #[must_use]
    pub fn local_structure(mut self, structure: LocalStructure) -> Self {
        self.structure = structure;
        self
    }

    /// Requests distinct directory classes on aliased platforms.
    ///
    /// This is a deliberate per-application choice, set once on the
    /// builder: every platform resolution from this resolver then
    /// appends the class segment beneath the application directory
    /// on Windows and macOS, whose provider bases alias several
    /// classes onto one root. The default (unset) keeps the standard
    /// platform layout, in which aliased classes share one root.
    /// Linux bases already separate classes and are unaffected.
    #[must_use]
    pub fn distinct_directory_classes(mut self, enabled: bool) -> Self {
        self.distinct_directory_classes = enabled;
        self
    }

    /// Registers the version-control discoverer.
    ///
    /// Registration is the explicit opt-in to VCS discovery. Only one
    /// discoverer per genus is allowed.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError::DuplicateDiscoverer`] when a VCS
    /// discoverer is already registered.
    pub fn register_vcs<D>(
        mut self,
        discoverer: D,
    ) -> Result<Self, ResolutionError>
    where
        D: VcsDiscoverer + 'static,
    {
        if self.vcs.is_some() {
            return Err(ResolutionError::DuplicateDiscoverer(
                DiscovererGenus::Vcs,
            ));
        }
        self.vcs = Some(Box::new(discoverer));
        Ok(self)
    }

    /// Registers the development-marker discoverer.
    ///
    /// Registration is the explicit opt-in to devfile discovery. Only
    /// one discoverer per genus is allowed.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError::DuplicateDiscoverer`] when a devfile
    /// discoverer is already registered.
    pub fn register_devfile<D>(
        mut self,
        discoverer: D,
    ) -> Result<Self, ResolutionError>
    where
        D: DevfileDiscoverer + 'static,
    {
        if self.devfile.is_some() {
            return Err(ResolutionError::DuplicateDiscoverer(
                DiscovererGenus::Devfile,
            ));
        }
        self.devfile = Some(Box::new(discoverer));
        Ok(self)
    }

    /// Sets one discoverer argument for a genus.
    ///
    /// Arguments are validated when a resolution runs, before any
    /// discoverer executes. Setting an argument never registers a
    /// discoverer on its own.
    #[must_use]
    pub fn discoverer_argument(
        mut self,
        genus: DiscovererGenus,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.arguments
            .entry(genus)
            .or_default()
            .insert(key.into(), value.into());
        self
    }

    /// Resolves one class through the documented precedence chain:
    /// explicit root, then discovered root, then platform.
    ///
    /// Relative explicit roots join against `start`. Discovery runs
    /// only through registered discoverers; with none registered, an
    /// eligible resolution falls through to the platform tier.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError`] for invalid inputs, invalid
    /// discoverer arguments, identity mismatches, unsupported
    /// topologies, and unavailable platform roots. Genuine absence of
    /// a repository falls through to the platform.
    pub fn resolve(
        &self,
        class: DirectoryClass,
        explicit: Option<&Path>,
        start: Option<&Path>,
    ) -> Result<ResolvedRoot, ResolutionError> {
        if let Some(explicit) = explicit {
            return self.resolve_explicit(explicit, start);
        }
        self.validate_arguments()?;
        if let Some(base) = self.run_discovery(start)? {
            return local_root(
                &ResolvedRoot {
                    path: base.path,
                    provenance: base.provenance,
                },
                class,
                &self.application,
                &self.structure,
            );
        }
        crate::platform::platform_root_with_classes(
            &self.application,
            class,
            self.distinct_directory_classes,
        )
    }

    /// Validates every argument bag before any discovery executes.
    fn validate_arguments(&self) -> Result<(), ResolutionError> {
        for (genus, bag) in &self.arguments {
            let registered = match genus {
                DiscovererGenus::Vcs => {
                    self.vcs.as_ref().map(|vcs| vcs.validate_args(bag))
                },
                DiscovererGenus::Devfile => {
                    self.devfile.as_ref().map(|dev| dev.validate_args(bag))
                },
            };
            match registered {
                // No discoverer registered for this genus: the bag
                // itself is the error, naming its first entry.
                None => {
                    let (key, value) = bag
                        .iter()
                        .next()
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .unwrap_or_default();
                    return Err(ResolutionError::InvalidDiscovererArgument {
                        discoverer: genus.label().to_string(),
                        key,
                        value,
                    });
                },
                Some(validation) => validation?,
            }
        }
        Ok(())
    }

    /// Runs the registered discoverers in genus order.
    ///
    /// A VCS base proceeds to devfile validation when a devfile
    /// discoverer is registered; genuine VCS absence ends discovery
    /// without a standalone devfile search.
    fn run_discovery(
        &self,
        start: Option<&Path>,
    ) -> Result<Option<DiscoveredBase>, ResolutionError> {
        if self.vcs.is_none() && self.devfile.is_none() {
            return Ok(None);
        }
        let Some(start) = start else {
            return Ok(None);
        };
        if let Some(vcs) = &self.vcs {
            let bag = self.bag(DiscovererGenus::Vcs);
            match vcs.discover(start, &bag)? {
                Some(base) => {
                    if let Some(devfile) = &self.devfile {
                        devfile.validate_at_base(
                            &base.path,
                            &self.bag(DiscovererGenus::Devfile),
                            &self.application,
                        )?;
                    }
                    return Ok(Some(base));
                },
                // Genuine absence: the discovered tier is absent.
                // No standalone devfile search runs on this path.
                None => return Ok(None),
            }
        }
        let Some(devfile) = &self.devfile else {
            return Ok(None);
        };
        devfile.discover_standalone(
            start,
            &self.bag(DiscovererGenus::Devfile),
            &self.application,
        )
    }

    /// Returns the argument bag for a genus (possibly empty).
    fn bag(&self, genus: DiscovererGenus) -> BTreeMap<String, String> {
        self.arguments.get(&genus).cloned().unwrap_or_default()
    }

    fn resolve_explicit(
        &self,
        explicit: &Path,
        start: Option<&Path>,
    ) -> Result<ResolvedRoot, ResolutionError> {
        if explicit.as_os_str().is_empty() {
            return Err(ResolutionError::EmptyPath);
        }
        validate_root_shape(explicit)?;
        let path: PathBuf = if explicit.is_absolute() {
            explicit.to_path_buf()
        } else {
            let Some(start) = start else {
                return Err(ResolutionError::RelativeStartDirectory(
                    explicit.to_path_buf(),
                ));
            };
            if !start.is_absolute() {
                return Err(ResolutionError::RelativeStartDirectory(
                    start.to_path_buf(),
                ));
            }
            // Preserve `..` semantically: lexical popping would
            // retarget paths through symlinked components.
            start.join(explicit)
        };
        Ok(ResolvedRoot {
            path,
            provenance: Provenance::CallerProvided,
        })
    }
}
