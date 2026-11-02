//! Directory locations for application caches, configuration, and state.
//!
//! The crate resolves candidate paths only. It never creates, modifies,
//! deletes, or locks anything, never parses or merges consumer
//! configuration, and never enforces ownership or permissions.
//! `Cargo.toml` identity parsing is the sole declared exception.

pub mod development;
pub mod git;
pub mod layers;
pub mod platform;
pub mod types;

pub use development::{DevelopmentLayout, development_root};
pub use git::{discover_checkout, validate_cargo_package};
pub use layers::LayeredRoots;
pub use platform::{platform_root, sanitize_name};
pub use types::{
    DevelopmentScope, DirectoryKind, GitCheckout, Provenance, ResolutionError,
    ResolvedRoot, validate_token,
};

use std::path::{Path, PathBuf};

/// Resolves one kind beneath an explicitly supplied directory.
///
/// # Errors
///
/// Returns [`ResolutionError::RelativeStartDirectory`] when `base` is
/// not absolute and [`ResolutionError::EmptyPath`] when it is empty.
pub fn supplied_root(
    base: &Path,
    kind: DirectoryKind,
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
        path: base.join(kind.segment()),
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
#[derive(Clone, Debug)]
pub struct Resolver {
    application: String,
    development_enabled: bool,
    layout: DevelopmentLayout,
}

impl Resolver {
    /// Creates a resolver for `application`.
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
            development_enabled: false,
            layout: DevelopmentLayout::default(),
        })
    }

    /// Enables or disables development-tree resolution.
    #[must_use]
    pub fn development_mode(mut self, enabled: bool) -> Self {
        self.development_enabled = enabled;
        self
    }

    /// Replaces the development layout.
    #[must_use]
    pub fn layout(mut self, layout: DevelopmentLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Resolves one kind through the documented precedence chain:
    /// explicit root, then eligible development root, then platform.
    ///
    /// Relative explicit roots join against `start`. Development roots
    /// require development mode and a discoverable checkout at `start`;
    /// identity validation runs when `expected_package` is given and a
    /// mismatch is an error, never a fallthrough.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError`] for invalid inputs, identity
    /// mismatches, unsupported topologies, and unavailable platform
    /// roots. Absence of a repository falls through to the platform.
    pub fn resolve(
        &self,
        kind: DirectoryKind,
        explicit: Option<&Path>,
        start: Option<&Path>,
        scope: DevelopmentScope,
        expected_package: Option<&str>,
    ) -> Result<ResolvedRoot, ResolutionError> {
        if let Some(explicit) = explicit {
            return self.resolve_explicit(explicit, start);
        }
        if self.development_enabled
            && let Some(start) = start
        {
            match discover_checkout(start) {
                Ok(checkout) => {
                    if let Some(expected) = expected_package {
                        validate_cargo_package(&checkout, expected)?;
                    }
                    return development_root(
                        &checkout,
                        scope,
                        kind,
                        &self.application,
                        &self.layout,
                    );
                },
                // Only genuine absence of a repository falls through;
                // execution failures surface to the caller.
                Err(ResolutionError::NotRepository(_)) => {},
                Err(error) => return Err(error),
            }
        }
        platform_root(&self.application, kind)
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
