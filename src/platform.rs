//! Platform-standard roots through the `directories` primitive.

use std::path::PathBuf;

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

/// Environment inputs captured exactly once for one resolution.
///
/// Both the chosen base and its provenance derive from this snapshot;
/// nothing rereads the live environment afterwards, and the process
/// environment is never mutated around a provider call.
#[derive(Clone, Debug)]
struct PlatformSnapshot {
    /// Raw XDG values per kind (Linux override inputs).
    xdg: [Option<PathBuf>; 4],
    /// Captured home directory, when any.
    home: Option<PathBuf>,
}

impl PlatformSnapshot {
    fn capture(base_dirs: Option<&BaseDirs>) -> Self {
        // Native bytes end to end: no lossy string conversion, so
        // a non-UTF8 absolute XDG root survives intact.
        let read = |name: &str| {
            std::env::var_os(name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        };
        Self {
            xdg: [
                read("XDG_CONFIG_HOME"),
                read("XDG_STATE_HOME"),
                read("XDG_CACHE_HOME"),
                read("XDG_DATA_HOME"),
            ],
            home: base_dirs.map(|dirs| dirs.home_dir().to_path_buf()),
        }
    }

    /// Derives the base and provenance for one kind from the snapshot.
    ///
    /// On Linux a valid absolute XDG value is the override; the
    /// default derives from the captured home, never from a provider
    /// value that may already embed a different XDG read. Elsewhere
    /// the provider default stands (its environment reads are inputs
    /// to the default, not overrides).
    fn base(
        &self,
        kind: DirectoryKind,
        provider_default: Option<PathBuf>,
    ) -> (Option<PathBuf>, Provenance) {
        if cfg!(target_os = "linux") {
            let index = match kind {
                DirectoryKind::Configuration => 0,
                DirectoryKind::State => 1,
                DirectoryKind::Cache => 2,
                DirectoryKind::Data => 3,
            };
            if let Some(candidate) = &self.xdg[index]
                && candidate.is_absolute()
            {
                return (
                    Some(candidate.clone()),
                    Provenance::PlatformEnvironment,
                );
            }
            let default = self.home.as_ref().map(|home| match kind {
                DirectoryKind::Configuration => home.join(".config"),
                DirectoryKind::State => home.join(".local/state"),
                DirectoryKind::Cache => home.join(".cache"),
                DirectoryKind::Data => home.join(".local/share"),
            });
            return (default, Provenance::PlatformDefault);
        }
        (provider_default, Provenance::PlatformDefault)
    }
}

/// State base on platforms whose provider has no state directory:
/// local application data on Windows, application support on macOS.
/// These are the bases `platformdirs` uses for state there.
fn state_base_fallback(dirs: &BaseDirs) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        Some(dirs.data_local_dir().to_path_buf())
    }
    #[cfg(target_os = "macos")]
    {
        Some(dirs.data_dir().to_path_buf())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = dirs;
        None
    }
}

/// Resolves one platform-standard root with honest provenance.
///
/// The validated application name is appended literally to the
/// captured platform base.
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
    let base_dirs = BaseDirs::new();
    let snapshot = PlatformSnapshot::capture(base_dirs.as_ref());
    let provider_default = base_dirs.as_ref().and_then(|dirs| match kind {
        DirectoryKind::Configuration => Some(dirs.config_dir().to_path_buf()),
        DirectoryKind::State => dirs
            .state_dir()
            .map(|path| path.to_path_buf())
            .or_else(|| state_base_fallback(dirs)),
        DirectoryKind::Cache => Some(dirs.cache_dir().to_path_buf()),
        DirectoryKind::Data => Some(dirs.data_dir().to_path_buf()),
    });
    let (base, provenance) = snapshot.base(kind, provider_default);
    let Some(base) = base.filter(|base| base.is_absolute()) else {
        // A relative captured home or provider base must never
        // propagate as a relative platform root.
        return Err(ResolutionError::Unavailable(kind));
    };
    Ok(ResolvedRoot {
        path: base.join(application),
        provenance,
    })
}
