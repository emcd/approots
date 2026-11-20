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
    DirectoryClass, DiscovererGenus, PartialRoots, Provenance,
    ResolutionError, ResolvedRoot, ResolvedRoots, validate_token,
};

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

/// Counts declared-environment reads, test builds only.
///
/// Integration tests cannot observe reads (a stable environment
/// makes recapture indistinguishable from single capture), and the
/// count must not become public API surface, so the counter lives
/// behind `cfg(test)` with exactly one inline test below.
#[cfg(test)]
static ENV_CAPTURE_READS: AtomicU64 = AtomicU64::new(0);

use crate::types::{refuse_windows_ambient_shape, validate_root_shape};

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
        provenance: Provenance::ExplicitBase,
    })
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
    env: Vec<(DirectoryClass, EnvShape, String)>,
}

/// What a declared environment value holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnvShape {
    /// An exact directory, validated like the explicit tier.
    Directory,
    /// A base for layout composition, validated like a supplied base.
    Base,
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
            env: self.env.clone(),
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
            env: Vec::new(),
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

    /// Declares an environment variable holding an exact directory
    /// for a class.
    ///
    /// Declaration is opt-in and per class, with one variable per
    /// shape: undeclared classes resolve exactly as without any
    /// declaration. The variable is read once per resolve with
    /// native preservation; an unset value reads as absent while a
    /// zero-length value is malformed (`EmptyPath`), and any other
    /// value validates exactly like the explicit tier. Values
    /// validate after capture, at selection time before any tier is
    /// chosen. Declaration problems (an unusable name, or a second
    /// declaration with the same shape for one class) are typed
    /// refusals raised at capture, before any tier resolves.
    #[must_use]
    pub fn env_directory(
        mut self,
        class: DirectoryClass,
        variable: impl Into<String>,
    ) -> Self {
        self.env.push((class, EnvShape::Directory, variable.into()));
        self
    }

    /// Declares an environment variable holding a layout base for a
    /// class.
    ///
    /// Behaves like [`Resolver::env_directory`], except the value is
    /// a base for layout composition (validated like a supplied
    /// base) rather than an exact directory.
    #[must_use]
    pub fn env_base(
        mut self,
        class: DirectoryClass,
        variable: impl Into<String>,
    ) -> Self {
        self.env.push((class, EnvShape::Base, variable.into()));
        self
    }

    /// Resolves one class through the documented precedence chain:
    /// explicit root, then declared environment value, then
    /// discovered root, then platform.
    ///
    /// Relative explicit roots join against `start`. A caller-supplied
    /// explicit root masks that class's declared variable entirely.
    /// Discovery runs only through registered discoverers; with none
    /// registered, an eligible resolution falls through to the
    /// platform tier.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError`] for invalid inputs, invalid
    /// environment declarations, invalid discoverer arguments,
    /// identity mismatches, unsupported topologies, and unavailable
    /// platform roots. Genuine absence of a repository falls through
    /// to the platform.
    pub fn resolve(
        &self,
        class: DirectoryClass,
        explicit: Option<&Path>,
        start: Option<&Path>,
    ) -> Result<ResolvedRoot, ResolutionError> {
        if let Some(explicit) = explicit {
            return self.resolve_explicit(explicit, start);
        }
        let captured = self.capture_env(std::slice::from_ref(&class))?;
        // The environment barrier runs before argument validation;
        // argument validation still runs when the environment wins.
        let env_root = self.apply_env(class, start, &captured)?;
        self.validate_arguments()?;
        if let Some(root) = env_root {
            return Ok(root);
        }
        self.resolve_tier(class, start)
    }

    /// Resolves one class past the explicit and environment tiers.
    ///
    /// The caller owns capture and argument validation; this runs
    /// discovery from `start` when discoverers are registered and
    /// otherwise falls back to the platform tier.
    fn resolve_tier(
        &self,
        class: DirectoryClass,
        start: Option<&Path>,
    ) -> Result<ResolvedRoot, ResolutionError> {
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

    /// Captures the declared environment for the given classes.
    ///
    /// Each captured class contributes its shape and value. Each
    /// distinct variable name is read exactly once with native
    /// preservation, and the cached value (including absence) is
    /// reused for every declaration naming it; per-class, per-shape
    /// validation and provenance still apply independently. Declaring
    /// the same shape twice for one class is a duplicate; declaring
    /// both shapes is allowed.
    /// Declaration errors surface here, before any tier resolves;
    /// classes without a declaration contribute nothing. Cache keys
    /// are the exact declared names. Windows may resolve case
    /// aliases to the same OS variable, but different declared
    /// spellings are separate cache keys and separate reads; no
    /// equality guarantee across reads under mutation. One read per
    /// name per capture is guaranteed; an atomic process-wide
    /// snapshot is not claimed.
    fn capture_env(
        &self,
        classes: &[DirectoryClass],
    ) -> Result<
        Vec<(DirectoryClass, EnvShape, Option<OsString>)>,
        ResolutionError,
    > {
        let mut captured = Vec::with_capacity(classes.len());
        let mut read: Vec<(String, Option<OsString>)> = Vec::new();
        for class in classes {
            // At most one declaration per (class, shape): same-shape
            // repeats are duplicates, while the two shapes may
            // coexist with the exact directory taking precedence.
            let mut seen = [false, false];
            let mut declarations = Vec::new();
            for (_, shape, variable) in
                self.env.iter().filter(|(declared, _, _)| declared == class)
            {
                let slot = match shape {
                    EnvShape::Directory => &mut seen[0],
                    EnvShape::Base => &mut seen[1],
                };
                if *slot {
                    return Err(ResolutionError::DuplicateEnvDeclaration(
                        *class,
                    ));
                }
                *slot = true;
                Self::validate_env_name(variable)?;
                declarations.push((*shape, variable));
            }
            for (shape, variable) in declarations {
                let value = if let Some((_, cached)) =
                    read.iter().find(|(name, _)| name == variable)
                {
                    cached.clone()
                } else {
                    let value = std::env::var_os(variable);
                    #[cfg(test)]
                    ENV_CAPTURE_READS.fetch_add(1, Ordering::SeqCst);
                    read.push((variable.clone(), value.clone()));
                    value
                };
                captured.push((*class, shape, value));
            }
        }
        Ok(captured)
    }

    /// Rejects unusable environment variable names.
    ///
    /// `std::env::var_os` reports empty names and names containing
    /// `=` or NUL as absent (`None`) rather than refusing them, so
    /// without this check a malformed declaration would silently
    /// fall through instead of failing loudly. Declarations are
    /// checked before any read.
    fn validate_env_name(variable: &str) -> Result<(), ResolutionError> {
        let problem = if variable.is_empty() {
            "name is empty"
        } else if variable.contains('=') {
            "name contains '='"
        } else if variable.contains('\0') {
            "name contains NUL"
        } else {
            return Ok(());
        };
        Err(ResolutionError::InvalidEnvDeclaration {
            name: variable.to_string(),
            reason: problem,
        })
    }

    /// Applies a captured environment value for one class, if any.
    ///
    /// An unset value reads as absent; a zero-length value is
    /// malformed. Any other value validates exactly like the
    /// matching explicit shape. When both shapes are declared, both
    /// validate first and the exact directory wins; an invalid value
    /// of either shape is a typed refusal rather than a fallback to
    /// the other. Validation is a pure function of the captured
    /// bytes, so re-applying one capture always agrees with itself.
    fn apply_env(
        &self,
        class: DirectoryClass,
        start: Option<&Path>,
        captured: &[(DirectoryClass, EnvShape, Option<OsString>)],
    ) -> Result<Option<ResolvedRoot>, ResolutionError> {
        // Validate every declared value for the class before
        // choosing: the winner never masks another shape's refusal.
        let mut built: Vec<(EnvShape, ResolvedRoot)> = Vec::new();
        for (_, shape, value) in captured
            .iter()
            .filter(|(declared, _, _)| *declared == class)
        {
            let Some(value) = value.as_ref() else {
                continue;
            };
            if value.is_empty() {
                // Omissibility is not nullability: an explicitly
                // set empty value is malformed, not absent. This
                // catches accidental `VAR=` exports (for example
                // from unset shell expansions without `set -u`)
                // instead of silently falling through to another
                // tier.
                return Err(ResolutionError::EmptyPath);
            }
            let value = PathBuf::from(value);
            match shape {
                EnvShape::Directory => built.push((
                    *shape,
                    ResolvedRoot {
                        path: self.joined_path(&value, start)?,
                        provenance: Provenance::AppvarDirectory,
                    },
                )),
                EnvShape::Base => {
                    let base = self.joined_path(&value, start)?;
                    built.push((
                        *shape,
                        local_root(
                            &ResolvedRoot {
                                path: base,
                                provenance: Provenance::AppvarBase,
                            },
                            class,
                            &self.application,
                            &self.structure,
                        )?,
                    ));
                },
            }
        }
        // The exact directory outranks a layout base when both are
        // usable; absence of both falls through to the next tier.
        if let Some((_, root)) = built
            .iter()
            .find(|(shape, _)| *shape == EnvShape::Directory)
        {
            return Ok(Some(root.clone()));
        }
        Ok(built.into_iter().next().map(|(_, root)| root))
    }

    /// Resolves a whole family with per-kind complete-or-missing roots.
    ///
    /// Each `Some` value is a complete directory for its class,
    /// validated exactly like the explicit tier (absolute and
    /// non-empty; Windows ambient-dependent shapes refused; a
    /// relative value joins against `start`, and relative with no
    /// start is a typed error) and reported `ExplicitDirectory`, never
    /// composed further. A `Some` value beats that kind's declared
    /// environment variable, which is never read. Each `None` value
    /// runs the normal chain for its class, including the declared
    /// environment tier: discovery from `start` when discoverers are
    /// registered, otherwise the platform tier — a missing start
    /// falls back to the environment tier when its value is
    /// absolute, otherwise to platform when the variable is unset,
    /// and never consults the ambient working directory. Malformed
    /// values (relative with no `start`, ambient-dependent shapes,
    /// zero-length) are typed refusals, never fallbacks. Validation
    /// runs in ordered phases: every
    /// `Some` field, then every eligible gap environment value, then
    /// discoverer arguments — before any gap discovery or platform
    /// work. All-filled input validates every field and returns the
    /// bundle without consulting the environment or validating
    /// arguments at all.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError`] for invalid supplied roots,
    /// invalid environment declarations, invalid discoverer
    /// arguments, identity mismatches, unsupported topologies, and
    /// unavailable platform roots, exactly as the per-kind path
    /// reports them.
    pub fn resolve_partially_explicit(
        &self,
        partial: &PartialRoots,
        start: Option<&Path>,
    ) -> Result<ResolvedRoots, ResolutionError> {
        // Per-kind fields in fixed order; plain public fields, no
        // accessors.
        let fields = [
            (
                DirectoryClass::Configuration,
                partial.configuration.as_ref(),
            ),
            (DirectoryClass::State, partial.state.as_ref()),
            (DirectoryClass::Cache, partial.cache.as_ref()),
            (DirectoryClass::Data, partial.data.as_ref()),
        ];
        // Validate every supplied root first, through the explicit
        // tier exactly: shape, Windows ambient, relative join. No
        // gap-filling, environment capture, discovery, or argument
        // validation runs before this loop completes.
        let mut resolved: [Option<ResolvedRoot>; 4] = [None, None, None, None];
        for ((_, field), slot) in fields.iter().zip(resolved.iter_mut()) {
            if let Some(path) = field {
                *slot = Some(self.resolve_explicit(path, start)?);
            }
        }
        if resolved.iter().all(Option::is_some) {
            // All-filled: pure validation. The environment is never
            // captured or consulted, and arguments are never
            // validated.
            let [configuration, state, cache, data] = resolved;
            let bundle = ResolvedRoots {
                configuration: configuration.expect("validated above"),
                state: state.expect("validated above"),
                cache: cache.expect("validated above"),
                data: data.expect("validated above"),
            };
            return Ok(bundle);
        }
        // Capture the declared environment once for the gap kinds
        // only; `Some` kinds mask their declarations entirely.
        let gaps: Vec<DirectoryClass> = fields
            .iter()
            .zip(resolved.iter())
            .filter(|(_, slot)| slot.is_none())
            .map(|((class, _), _)| *class)
            .collect();
        let captured = self.capture_env(&gaps)?;
        // The environment barrier: every eligible gap value validates
        // before any gap discovery or platform work runs. Invalid
        // values are never masked by another kind's valid root.
        for class in &gaps {
            self.apply_env(*class, start, &captured)?;
        }
        // Argument validation runs even when the environment tier
        // will supply gap roots.
        self.validate_arguments()?;
        // Fill each gap from the shared capture: environment, then
        // discovery, then platform. No recapture, no repeated
        // argument validation.
        for ((class, _), slot) in fields.iter().zip(resolved.iter_mut()) {
            if slot.is_none() {
                if let Some(root) = self.apply_env(*class, start, &captured)? {
                    *slot = Some(root);
                } else {
                    *slot = Some(self.resolve_tier(*class, start)?);
                }
            }
        }
        let [configuration, state, cache, data] = resolved;
        let bundle = ResolvedRoots {
            configuration: configuration.expect("filled above"),
            state: state.expect("filled above"),
            cache: cache.expect("filled above"),
            data: data.expect("filled above"),
        };
        Ok(bundle)
    }

    /// Validates every argument bag before any discovery executes.
    ///
    /// Bags for unregistered genera are rejected first; then every
    /// registered discoverer's effective bag is validated, including
    /// an implicitly empty bag, so a discoverer that requires an
    /// option refuses before its discovery ever runs.
    fn validate_arguments(&self) -> Result<(), ResolutionError> {
        for (genus, bag) in &self.arguments {
            let registered = match genus {
                DiscovererGenus::Vcs => self.vcs.is_some(),
                DiscovererGenus::Devfile => self.devfile.is_some(),
            };
            if !registered {
                // No discoverer registered for this genus: the bag
                // itself is the error, naming its first entry.
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
            }
        }
        if let Some(vcs) = &self.vcs {
            vcs.validate_args(&self.bag(DiscovererGenus::Vcs))?;
        }
        if let Some(devfile) = &self.devfile {
            devfile.validate_args(&self.bag(DiscovererGenus::Devfile))?;
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
        Ok(ResolvedRoot {
            path: self.joined_path(explicit, start)?,
            provenance: Provenance::ExplicitDirectory,
        })
    }

    /// Validates an explicit-shape path: absolute, or relative joined
    /// against an absolute `start` with `..` preserved semantically.
    fn joined_path(
        &self,
        explicit: &Path,
        start: Option<&Path>,
    ) -> Result<PathBuf, ResolutionError> {
        if explicit.as_os_str().is_empty() {
            return Err(ResolutionError::EmptyPath);
        }
        // Windows ambient-dependent shapes are refused before the
        // ordinary-relative join, in either absoluteness branch: they
        // must never silently join against the start.
        refuse_windows_ambient_shape(explicit)?;
        if explicit.is_absolute() {
            validate_root_shape(explicit)?;
            Ok(explicit.to_path_buf())
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
            Ok(start.join(explicit))
        }
    }
}

#[cfg(test)]
mod env_capture_tests {
    use std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
        sync::{Arc, Mutex, atomic::Ordering},
    };

    use super::ENV_CAPTURE_READS;
    use crate::{
        DirectoryClass, DiscoveredBase, Discoverer, PartialRoots, Provenance,
        ResolutionError, Resolver, VcsDiscoverer,
    };

    /// Records invocations alongside the read count at call time,
    /// so the capture-once test can prove every read completed
    /// before the first downstream discovery call.
    #[derive(Debug, Clone, Default)]
    struct RecordingDiscoverer {
        calls: Arc<Mutex<Vec<(PathBuf, u64)>>>,
    }

    impl Discoverer for RecordingDiscoverer {
        fn genus(&self) -> crate::DiscovererGenus {
            crate::DiscovererGenus::Vcs
        }
    }

    impl VcsDiscoverer for RecordingDiscoverer {
        fn validate_args(
            &self,
            _args: &BTreeMap<String, String>,
        ) -> Result<(), ResolutionError> {
            Ok(())
        }

        fn discover(
            &self,
            start: &Path,
            _args: &BTreeMap<String, String>,
        ) -> Result<Option<DiscoveredBase>, ResolutionError> {
            // Recorded alongside the call: all environment reads
            // must already be done when the first discovery runs.
            let at = ENV_CAPTURE_READS.load(Ordering::SeqCst);
            self.calls
                .lock()
                .expect("calls")
                .push((start.to_path_buf(), at));
            Ok(Some(DiscoveredBase {
                path: start.to_path_buf(),
                provenance: Provenance::Discovery,
            }))
        }

        fn clone_box(&self) -> Box<dyn VcsDiscoverer> {
            Box::new(self.clone())
        }
    }

    fn reads() -> u64 {
        ENV_CAPTURE_READS.load(Ordering::SeqCst)
    }

    static STATE_VAR: &str = "APPROOTS_TEST_CAPTURE_STATE";
    static CACHE_VAR: &str = "APPROOTS_TEST_CAPTURE_CACHE";
    static BOTH_VAR: &str = "APPROOTS_TEST_CAPTURE_BOTH";
    static START_VAR: &str = "APPROOTS_TEST_CAPTURE_START";
    static CHILD_FLAG: &str = "APPROOTS_CAPTURE_CHILD";

    // The single inline test this crate carries: capture-once is
    // unobservable through any public contract under a stable
    // environment, so it needs the test-only read counter. Exactly
    // one `#[test]` lives in this binary, and the counter plus the
    // discoverer seam are `cfg(test)`-only with zero production
    // impact. All fixture environments are owned by a guarded
    // self-child (safe `Command::env` only): the runner process
    // environment is never mutated, so no `unsafe` env writes exist
    // anywhere in this test.
    #[test]
    fn env_capture_reads_once_per_class() {
        use std::process::Command;
        if std::env::var(CHILD_FLAG).as_deref() == Ok("1") {
            child_body();
            return;
        }
        let exe = std::env::current_exe().expect("test exe");
        let unique = std::process::id();
        let output = Command::new(&exe)
            .args([
                "env_capture_tests::env_capture_reads_once_per_class",
                "--exact",
                "--nocapture",
            ])
            .env(CHILD_FLAG, "1")
            .env(STATE_VAR, capture_dir(&unique, "state"))
            .env(CACHE_VAR, capture_dir(&unique, "cache"))
            .env(BOTH_VAR, capture_dir(&unique, "both"))
            .env(START_VAR, capture_dir(&unique, "start"))
            .output()
            .expect("spawn capture child");
        assert!(
            output.status.success(),
            "capture child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Native absolute fixture paths: absolute on every platform,
    /// unlike hardcoded `/tmp/...` literals.
    fn capture_dir(unique: &u32, tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("approots-capture-{tag}-{unique}"))
    }

    fn child_body() {
        ENV_CAPTURE_READS.store(0, Ordering::SeqCst);
        let start =
            PathBuf::from(std::env::var_os(START_VAR).expect("start var"));
        let recorder = RecordingDiscoverer::default();
        let discovered = recorder.calls.clone();
        let resolver = Resolver::new("fixture-app")
            .expect("resolver")
            .register_vcs(recorder)
            .expect("register vcs")
            .env_directory(DirectoryClass::State, STATE_VAR)
            .env_directory(DirectoryClass::Cache, CACHE_VAR);
        // State and Cache gaps fill from the environment; the
        // remaining gaps run real discovery afterwards.
        let bundle = resolver
            .resolve_partially_explicit(&PartialRoots::default(), Some(&start))
            .expect("family resolves");
        assert_eq!(bundle.state.provenance, Provenance::AppvarDirectory);
        assert_eq!(bundle.cache.provenance, Provenance::AppvarDirectory);
        // Exactly one read per declared gap class...
        assert_eq!(reads(), 2);
        // ...all completed before the first discovery call, which
        // ran afterwards without triggering any re-read.
        let calls = discovered.lock().expect("calls");
        assert!(
            !calls.is_empty(),
            "discovery must run for the undeclared gaps"
        );
        assert!(
            calls.iter().all(|(_, at)| *at == 2),
            "every read completes before discovery starts"
        );
        assert_eq!(reads(), 2, "no recapture after downstream work");
        drop(calls);
        // A single-class resolve reads exactly once more...
        resolver
            .resolve(DirectoryClass::State, None, None)
            .expect("single resolve");
        assert_eq!(reads(), 3);
        // ...while untouched classes are never read.
        resolver
            .resolve(DirectoryClass::Configuration, None, None)
            .expect("undeclared resolve");
        assert_eq!(reads(), 3);
        // A shared variable is read once and reused for both
        // classes, not once per class.
        let shared = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, STATE_VAR)
            .env_directory(DirectoryClass::Cache, STATE_VAR);
        let bundle = shared
            .resolve_partially_explicit(&PartialRoots::default(), None)
            .expect("shared variable resolves");
        assert_eq!(bundle.state.provenance, Provenance::AppvarDirectory);
        assert_eq!(bundle.cache.provenance, Provenance::AppvarDirectory);
        assert_eq!(reads(), 4, "one read per distinct name");
        // One name backing both shapes of one class is still a
        // single read, with the exact directory winning.
        let both = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::Configuration, BOTH_VAR)
            .env_base(DirectoryClass::Configuration, BOTH_VAR);
        let root = both
            .resolve(DirectoryClass::Configuration, None, None)
            .expect("shared shape variable resolves");
        assert_eq!(root.provenance, Provenance::AppvarDirectory);
        assert_eq!(reads(), 5, "one read for both shapes");
    }
}
