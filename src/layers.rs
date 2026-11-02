//! Ordered, closed directory layer candidates.

use std::{fs, io::ErrorKind, path::PathBuf};

use crate::types::{Provenance, ResolutionError, ResolvedRoot};

/// An ordered, non-empty list of absolute directory layers.
///
/// The list is closed: only the supplied layers are ever consulted.
/// Per-file lookup within the returned roots belongs to consumers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayeredRoots {
    layers: Vec<PathBuf>,
}

impl LayeredRoots {
    /// Builds a layer list from supplied roots, in precedence order.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError::EmptyPath`] for an empty list or an
    /// empty element, and [`ResolutionError::RelativeStartDirectory`]
    /// for a non-absolute layer.
    pub fn new(
        layers: impl IntoIterator<Item = PathBuf>,
    ) -> Result<Self, ResolutionError> {
        let layers: Vec<PathBuf> = layers.into_iter().collect();
        if layers.is_empty()
            || layers.iter().any(|layer| layer.as_os_str().is_empty())
        {
            return Err(ResolutionError::EmptyPath);
        }
        for layer in &layers {
            if !layer.is_absolute() {
                return Err(ResolutionError::RelativeStartDirectory(
                    layer.clone(),
                ));
            }
        }
        Ok(Self { layers })
    }

    /// Resolves the ordered directory candidates with provenance.
    ///
    /// A nonexistent layer is retained as a candidate; a root probe
    /// failing with anything other than not-found is a fault. Two
    /// layers canonicalizing to the same directory are an ambiguity
    /// error. Existence and per-file policy stay with consumers.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError::Unreadable`] for an unreadable layer
    /// and [`ResolutionError::Ambiguous`] for collapsed directories.
    pub fn resolve_roots(&self) -> Result<Vec<ResolvedRoot>, ResolutionError> {
        let mut resolved = Vec::with_capacity(self.layers.len());
        let mut canonical_seen: Vec<PathBuf> = Vec::new();
        for layer in &self.layers {
            match fs::metadata(layer) {
                Ok(_) => {
                    let canonical = fs::canonicalize(layer)
                        .unwrap_or_else(|_| layer.clone());
                    if canonical_seen.contains(&canonical) {
                        return Err(ResolutionError::Ambiguous(layer.clone()));
                    }
                    canonical_seen.push(canonical);
                },
                Err(error) if error.kind() == ErrorKind::NotFound => {},
                Err(_) => {
                    return Err(ResolutionError::Unreadable(layer.clone()));
                },
            }
            resolved.push(ResolvedRoot {
                path: layer.clone(),
                provenance: Provenance::CallerProvided,
            });
        }
        Ok(resolved)
    }
}
