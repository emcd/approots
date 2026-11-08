//! Local directory structure mated onto any base.
//!
//! A `LocalStructure` is a validated segment scheme. It is not tied to
//! development: the same structure mates onto a discovered root, a
//! supplied base, or a raw provider base through [`local_root`], which
//! inherits the base's provenance exactly. Rawness (not already
//! application-namespaced) is an unchecked caller precondition: a raw
//! base that itself ends in the application name still composes
//! literally. Shape is checked: empty, relative, and Windows
//! ambient-dependent bases are typed refusals.

use std::path::Path;

use crate::types::{
    DirectoryClass, ResolutionError, ResolvedRoot,
    validate_relative_components, validate_root_shape, validate_token,
};

/// Caller-overridable local structure segments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalStructure {
    /// Directory holding the local tree (default `.auxiliary`).
    pub auxiliary: std::path::PathBuf,
    /// Configuration segment (default `configuration`).
    pub configuration: std::path::PathBuf,
    /// State segment (default `state`).
    pub state: std::path::PathBuf,
    /// Cache segment (default `cache`).
    pub cache: std::path::PathBuf,
    /// Data segment (default `data`).
    pub data: std::path::PathBuf,
}

impl Default for LocalStructure {
    fn default() -> Self {
        Self {
            auxiliary: std::path::PathBuf::from(".auxiliary"),
            configuration: std::path::PathBuf::from("configuration"),
            state: std::path::PathBuf::from("state"),
            cache: std::path::PathBuf::from("cache"),
            data: std::path::PathBuf::from("data"),
        }
    }
}

impl LocalStructure {
    /// Validates every segment as relative components beneath a base.
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

    fn segment(&self, class: DirectoryClass) -> &Path {
        match class {
            DirectoryClass::Configuration => &self.configuration,
            DirectoryClass::State => &self.state,
            DirectoryClass::Cache => &self.cache,
            DirectoryClass::Data => &self.data,
        }
    }
}

/// Mates a local structure onto a base, inheriting its provenance.
///
/// The composition is exactly
/// `<base>/<prefix>/<class-segment>/<application>`; the application
/// segment is appended exactly once relative to the given base, even
/// when the base itself ends in the application name. Bases must be
/// absolute and non-empty, and on Windows must not use ambient
/// drive-relative or root-relative shapes: with no explicit-start
/// parameter, unresolved bases are typed refusals, not propagated
/// relative roots. A `platform_root` result is a complete root, never
/// a `local_root` base. Creates nothing. Valid bases are propagated
/// lexically; they are never canonicalized.
///
/// # Errors
///
/// Returns [`ResolutionError::EmptyPath`] for an empty base,
/// [`ResolutionError::RelativeStartDirectory`] for a relative base,
/// [`ResolutionError::InvalidName`] for an invalid application name,
/// structure segment, or Windows root shape.
pub fn local_root(
    base: &ResolvedRoot,
    class: DirectoryClass,
    application: &str,
    structure: &LocalStructure,
) -> Result<ResolvedRoot, ResolutionError> {
    validate_root_shape(&base.path)?;
    validate_token(application)?;
    structure.validate()?;
    let path = base
        .path
        .join(&structure.auxiliary)
        .join(structure.segment(class))
        .join(application);
    Ok(ResolvedRoot {
        path,
        provenance: base.provenance,
    })
}
