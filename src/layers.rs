//! Ordered, closed directory layer lists (first-match-wins per file).

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use crate::types::ResolutionError;

/// An ordered, non-empty list of directory layers, highest first.
///
/// The list is closed: only the supplied layers are ever consulted.
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
    /// empty element.
    pub fn new(
        layers: impl IntoIterator<Item = PathBuf>,
    ) -> Result<Self, ResolutionError> {
        let layers: Vec<PathBuf> = layers.into_iter().collect();
        if layers.is_empty()
            || layers.iter().any(|layer| layer.as_os_str().is_empty())
        {
            return Err(ResolutionError::EmptyPath);
        }
        Ok(Self { layers })
    }

    /// The layers, highest precedence first.
    #[must_use]
    pub fn layers(&self) -> &[PathBuf] {
        &self.layers
    }

    /// Finds a relative file in the first layer holding it.
    ///
    /// A nonexistent layer is retained as a candidate and supplies
    /// nothing; an unreadable layer (a root probe failing with anything
    /// other than not-found) is a fault. Two layers whose copies of the
    /// file canonicalize to the same target are an ambiguity error.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError::Unreadable`] for an unreadable layer
    /// and [`ResolutionError::Ambiguous`] for symlink-collapsed copies.
    pub fn resolve_file(
        &self,
        relative: &Path,
    ) -> Result<Option<PathBuf>, ResolutionError> {
        crate::types::validate_relative_components(relative)?;
        let mut found: Option<PathBuf> = None;
        let mut found_canonical: Option<PathBuf> = None;
        for layer in &self.layers {
            // Root-level probe distinguishes absence from unreadability.
            match fs::metadata(layer) {
                Ok(_) => {},
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    continue;
                },
                Err(_) => {
                    return Err(ResolutionError::Unreadable(layer.clone()));
                },
            }
            let candidate = layer.join(relative);
            match fs::metadata(&candidate) {
                Ok(metadata) if metadata.is_file() => {
                    let canonical = fs::canonicalize(&candidate)
                        .unwrap_or_else(|_| candidate.clone());
                    if let Some(previous) = &found_canonical
                        && *previous == canonical
                    {
                        return Err(ResolutionError::Ambiguous(candidate));
                    }
                    if found.is_none() {
                        found = Some(candidate);
                        found_canonical = Some(canonical);
                    }
                },
                Ok(_) => {
                    return Err(ResolutionError::Unreadable(candidate));
                },
                Err(error) if error.kind() == ErrorKind::NotFound => {},
                Err(_) => {
                    return Err(ResolutionError::Unreadable(candidate));
                },
            }
        }
        Ok(found)
    }
}
