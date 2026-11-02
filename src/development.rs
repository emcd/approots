//! Development-tree candidates beneath a checkout.

use std::path::{Path, PathBuf};

use crate::types::{
    DevelopmentScope, DirectoryKind, GitCheckout, Provenance, ResolutionError,
    ResolvedRoot, validate_relative_components, validate_token,
};

/// Caller-overridable development layout segments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevelopmentLayout {
    /// Directory holding the development tree (default `.auxiliary`).
    pub auxiliary: PathBuf,
    /// Configuration segment (default `configuration`).
    pub configuration: PathBuf,
    /// State segment (default `state`).
    pub state: PathBuf,
    /// Cache segment (default `cache`).
    pub cache: PathBuf,
    /// Data segment (default `data`).
    pub data: PathBuf,
}

impl Default for DevelopmentLayout {
    fn default() -> Self {
        Self {
            auxiliary: PathBuf::from(".auxiliary"),
            configuration: PathBuf::from("configuration"),
            state: PathBuf::from("state"),
            cache: PathBuf::from("cache"),
            data: PathBuf::from("data"),
        }
    }
}

impl DevelopmentLayout {
    /// Validates every segment as relative components beneath a root.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError::InvalidName`] for rooted, prefixed, or
    /// traversal segments.
    pub fn validate(&self) -> Result<(), ResolutionError> {
        for segment in [
            &self.auxiliary,
            &self.configuration,
            &self.state,
            &self.cache,
            &self.data,
        ] {
            validate_relative_components(segment)?;
        }
        Ok(())
    }

    fn segment(&self, kind: DirectoryKind) -> &Path {
        match kind {
            DirectoryKind::Configuration => &self.configuration,
            DirectoryKind::State => &self.state,
            DirectoryKind::Cache => &self.cache,
            DirectoryKind::Data => &self.data,
        }
    }
}

/// Resolves one development candidate. Creates nothing.
///
/// # Errors
///
/// Returns [`ResolutionError::InvalidName`] for an invalid application
/// name or layout segment.
pub fn development_root(
    checkout: &GitCheckout,
    scope: DevelopmentScope,
    kind: DirectoryKind,
    application: &str,
    layout: &DevelopmentLayout,
) -> Result<ResolvedRoot, ResolutionError> {
    validate_token(application)?;
    layout.validate()?;
    let (base, provenance) = match scope {
        DevelopmentScope::Common => (
            &checkout.common_worktree_root,
            Provenance::DevelopmentCommon,
        ),
        DevelopmentScope::Worktree => {
            (&checkout.worktree_root, Provenance::DevelopmentWorktree)
        },
    };
    let path = base
        .join(&layout.auxiliary)
        .join(layout.segment(kind))
        .join(application);
    Ok(ResolvedRoot { path, provenance })
}
