//! Platform-standard roots through the `directories` primitive.

use directories::ProjectDirs;

use crate::types::{DirectoryKind, Provenance, ResolutionError, ResolvedRoot};

/// Sanitizes a name for filename use (lossy, not injective).
#[must_use]
pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_alphanumeric()
                || character == '-'
                || character == '_'
            {
                character
            } else {
                '-'
            }
        })
        .collect()
}

/// Resolves one platform-standard root with honest provenance.
///
/// Provenance comes from the same captured environment inputs used for
/// resolution; an opaque provider result is never relabelled.
///
/// # Errors
///
/// Returns [`ResolutionError::Unavailable`] when the platform offers no
/// root for `kind` (including missing home configuration).
pub fn platform_root(
    application: &str,
    kind: DirectoryKind,
) -> Result<ResolvedRoot, ResolutionError> {
    let Some(directories) = ProjectDirs::from("", "", application) else {
        return Err(ResolutionError::Unavailable(kind));
    };
    let environment_variable = match kind {
        DirectoryKind::Configuration => "XDG_CONFIG_HOME",
        DirectoryKind::State => "XDG_STATE_HOME",
        DirectoryKind::Cache => "XDG_CACHE_HOME",
        DirectoryKind::Data => "XDG_DATA_HOME",
    };
    let from_environment = std::env::var_os(environment_variable)
        .is_some_and(|value| !value.is_empty());
    let provenance = if cfg!(target_os = "linux") && from_environment {
        Provenance::PlatformEnvironment
    } else {
        Provenance::PlatformDefault
    };
    let path = match kind {
        DirectoryKind::Configuration => {
            Some(directories.config_dir().to_path_buf())
        },
        DirectoryKind::State => {
            directories.state_dir().map(|path| path.to_path_buf())
        },
        DirectoryKind::Cache => Some(directories.cache_dir().to_path_buf()),
        DirectoryKind::Data => Some(directories.data_dir().to_path_buf()),
    };
    let Some(path) = path else {
        return Err(ResolutionError::Unavailable(kind));
    };
    Ok(ResolvedRoot { path, provenance })
}
