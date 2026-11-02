//! Platform-standard roots through the `directories` primitive.

use directories::BaseDirs;

use crate::types::{
    DirectoryKind, Provenance, ResolutionError, ResolvedRoot, validate_token,
};

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
/// Inputs are captured and validated once: on Linux a valid absolute
/// XDG override supplies both the base and `PlatformEnvironment`
/// provenance; relative or blank overrides are ignored by the
/// platform convention and yield the default. The validated
/// application name is appended literally to the platform base.
///
/// # Errors
///
/// Returns [`ResolutionError::InvalidName`] for an invalid application
/// name and [`ResolutionError::Unavailable`] when the platform offers
/// no root for `kind` (including missing home configuration).
pub fn platform_root(
    application: &str,
    kind: DirectoryKind,
) -> Result<ResolvedRoot, ResolutionError> {
    validate_token(application)?;
    let Some(base_dirs) = BaseDirs::new() else {
        return Err(ResolutionError::Unavailable(kind));
    };
    let (base, provenance) = platform_base(&base_dirs, kind)?;
    let Some(base) = base else {
        return Err(ResolutionError::Unavailable(kind));
    };
    Ok(ResolvedRoot {
        path: base.join(application),
        provenance,
    })
}

/// Captures the platform base for one kind from validated inputs once.
fn platform_base(
    base_dirs: &BaseDirs,
    kind: DirectoryKind,
) -> Result<(Option<std::path::PathBuf>, Provenance), ResolutionError> {
    if cfg!(target_os = "linux") {
        let environment_variable = match kind {
            DirectoryKind::Configuration => "XDG_CONFIG_HOME",
            DirectoryKind::State => "XDG_STATE_HOME",
            DirectoryKind::Cache => "XDG_CACHE_HOME",
            DirectoryKind::Data => "XDG_DATA_HOME",
        };
        if let Some(value) = std::env::var_os(environment_variable) {
            let candidate = std::path::PathBuf::from(&value);
            if !value.is_empty() && candidate.is_absolute() {
                return Ok((Some(candidate), Provenance::PlatformEnvironment));
            }
        }
    }
    let base = match kind {
        DirectoryKind::Configuration => {
            Some(base_dirs.config_dir().to_path_buf())
        },
        DirectoryKind::State => {
            base_dirs.state_dir().map(|path| path.to_path_buf())
        },
        DirectoryKind::Cache => Some(base_dirs.cache_dir().to_path_buf()),
        DirectoryKind::Data => Some(base_dirs.data_dir().to_path_buf()),
    };
    Ok((base, Provenance::PlatformDefault))
}
