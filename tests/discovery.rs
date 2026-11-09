//! Discovery state-machine and local-structure composition tests.

mod common;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use approots::{
    DevfileDiscovererCargo, DirectoryClass, DiscoveredBase, Discoverer,
    DiscovererGenus, LocalStructure, PartialRoots, Provenance,
    ResolutionError, Resolver, VcsDiscoverer, VcsDiscovererGit, local_root,
    validate_token,
};
use common::*;

#[test]
fn cargo_identity_accepts_matches_and_rejects_at_selected_base() {
    let repo = git_repo("identity");
    let resolve = |app: &str| {
        git_cargo_resolver(app).resolve(
            DirectoryClass::State,
            None,
            Some(&repo),
        )
    };
    let matched = resolve("fixture-app").expect("match");
    assert_eq!(matched.path, repo.join(".auxiliary/state/fixture-app"));
    assert_eq!(matched.provenance, Provenance::DevelopmentWorktree);
    assert!(matches!(
        resolve("other-app"),
        Err(ResolutionError::IdentityMismatch { .. })
    ));
    fs::write(repo.join("Cargo.toml"), "[workspace]\nmembers = []\n")
        .expect("virtual manifest");
    assert!(matches!(
        resolve("fixture-app"),
        Err(ResolutionError::VirtualWorkspace(_))
    ));
}

#[test]
fn cargo_expected_name_argument_overrides_application() {
    let repo = git_repo("expected-name");
    // The application and package names legitimately differ here.
    let resolver = cargo_resolver("custom-app").discoverer_argument(
        DiscovererGenus::Devfile,
        DevfileDiscovererCargo::EXPECTED_NAME_KEY,
        "fixture-app",
    );
    let resolved = resolver
        .resolve(DirectoryClass::State, None, Some(&repo))
        .expect("expected-name override");
    assert_eq!(resolved.path, repo.join(".auxiliary/state/custom-app"));
    // ...but the structure still carries the application name.
    let mismatched = cargo_resolver("fixture-app").discoverer_argument(
        DiscovererGenus::Devfile,
        DevfileDiscovererCargo::EXPECTED_NAME_KEY,
        "other-app",
    );
    assert!(matches!(
        mismatched.resolve(DirectoryClass::State, None, Some(&repo)),
        Err(ResolutionError::IdentityMismatch { .. })
    ));
}

#[test]
fn local_structure_mates_and_refuses_escape() {
    let base = temp_dir("localstruct");
    let structure = LocalStructure::default();
    let root = local_root(
        &approots::ResolvedRoot {
            path: base.clone(),
            provenance: Provenance::SuppliedDirectory,
        },
        DirectoryClass::State,
        "fixture-app",
        &structure,
    )
    .expect("local root");
    assert_eq!(root.path, base.join(".auxiliary/state/fixture-app"));
    assert_eq!(root.provenance, Provenance::SuppliedDirectory);
    assert!(!root.path.exists(), "resolution creates nothing");

    let bad = LocalStructure {
        auxiliary: PathBuf::from("../escape"),
        ..LocalStructure::default()
    };
    let base_root = approots::ResolvedRoot {
        path: base.clone(),
        provenance: Provenance::CallerProvided,
    };
    assert!(
        local_root(&base_root, DirectoryClass::State, "fixture-app", &bad)
            .is_err()
    );
    // Provenance is inherited, never invented.
    let caller = local_root(
        &approots::ResolvedRoot {
            path: base.clone(),
            provenance: Provenance::CallerProvided,
        },
        DirectoryClass::Cache,
        "fixture-app",
        &structure,
    )
    .expect("caller base");
    assert_eq!(caller.provenance, Provenance::CallerProvided);
    assert!(validate_token("a/b").is_err());
    assert!(validate_token("..").is_err());
    assert!(validate_token("good-name_1").is_ok());
}

#[test]
fn git_absence_skips_standalone_cargo_search() {
    // A plain directory with a matching Cargo.toml but no .git: the
    // registered Git discoverer reports genuine absence, so the
    // discovered tier is absent and the standalone Cargo search does
    // NOT run — the platform tier answers.
    let plain = temp_dir("no-repo");
    fs::write(
        plain.join("Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");
    let resolved = git_cargo_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&plain))
        .expect("absence falls through");
    assert!(matches!(
        resolved.provenance,
        Provenance::PlatformEnvironment | Provenance::PlatformDefault
    ));
    assert!(!resolved.path.join(".auxiliary").exists());
}

#[test]
fn devfile_only_search_finds_upward_and_absent_means_fallthrough() {
    let base = temp_dir("standalone");
    fs::write(
        base.join("Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");
    let nested = base.join("deep/nest");
    fs::create_dir_all(&nested).expect("nested dirs");
    // Only the devfile discoverer is registered: the standalone
    // upward search proposes the containing directory as the base.
    let resolved = cargo_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&nested))
        .expect("standalone search");
    assert_eq!(resolved.path, base.join(".auxiliary/state/fixture-app"));
    assert_eq!(resolved.provenance, Provenance::DevelopmentWorktree);
    // No marker anywhere: the tier is absent, not an error.
    let empty = temp_dir("standalone-empty");
    let fallen = cargo_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&empty))
        .expect("no marker falls through");
    assert!(matches!(
        fallen.provenance,
        Provenance::PlatformEnvironment | Provenance::PlatformDefault
    ));
}

#[test]
fn devfile_only_search_prefers_nearest_marker() {
    let base = temp_dir("nearest");
    fs::write(
        base.join("Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("base manifest");
    let mid = base.join("mid");
    fs::create_dir_all(&mid).expect("mid dir");
    fs::write(
        mid.join("Cargo.toml"),
        "[package]\nname = \"foreign-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("mid manifest");
    let leaf = mid.join("leaf");
    fs::create_dir_all(&leaf).expect("leaf dir");
    // The nearest marker wins, even when it mismatches.
    assert!(matches!(
        cargo_resolver("fixture-app").resolve(
            DirectoryClass::State,
            None,
            Some(&leaf)
        ),
        Err(ResolutionError::IdentityMismatch { .. })
    ));
    fs::remove_file(mid.join("Cargo.toml")).expect("remove mid marker");
    let resolved = cargo_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&leaf))
        .expect("base marker");
    assert_eq!(resolved.path, base.join(".auxiliary/state/fixture-app"));
}

#[test]
fn cargo_outcomes_at_selected_base_are_typed() {
    // Missing marker at a selected VCS base is a typed error.
    let repo = git_repo("cargo-missing");
    fs::remove_file(repo.join("Cargo.toml")).expect("remove manifest");
    assert!(matches!(
        git_cargo_resolver("fixture-app").resolve(
            DirectoryClass::State,
            None,
            Some(&repo)
        ),
        Err(ResolutionError::Unreadable(_))
    ));
    // Malformed TOML is undetermined, never a fallthrough.
    let repo = git_repo("cargo-malformed");
    fs::write(repo.join("Cargo.toml"), "[[[\n").expect("malformed");
    assert!(matches!(
        git_cargo_resolver("fixture-app").resolve(
            DirectoryClass::State,
            None,
            Some(&repo)
        ),
        Err(ResolutionError::UndeterminedPackage(_))
    ));
    // Unreadable manifest is a fault, not "no marker".
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let repo = git_repo("cargo-unreadable");
        let manifest = repo.join("Cargo.toml");
        fs::set_permissions(&manifest, fs::Permissions::from_mode(0o000))
            .expect("chmod manifest");
        let outcome = git_cargo_resolver("fixture-app").resolve(
            DirectoryClass::State,
            None,
            Some(&repo),
        );
        fs::set_permissions(&manifest, fs::Permissions::from_mode(0o644))
            .expect("restore manifest");
        assert!(matches!(outcome, Err(ResolutionError::Unreadable(_))));
    }
}

#[test]
fn scope_omitted_defaults_to_local_in_linked_worktree() {
    let repo = git_repo("scope-default");
    let linked = temp_dir("scope-default-wt").join("wt");
    let linked_arg = linked.to_string_lossy().to_string();
    git(&repo, &["worktree", "add", "-q", &linked_arg]);
    // No scope argument: behaves exactly like an explicit Local.
    let resolved = git_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&linked))
        .expect("default scope");
    assert_eq!(resolved.provenance, Provenance::DevelopmentWorktree);
    let expected =
        strip_verbatim_prefix(linked.canonicalize().expect("canonical"));
    assert_eq!(resolved.path, expected.join(".auxiliary/state/fixture-app"));
}

#[test]
fn invalid_discoverer_arguments_are_typed_before_discovery() {
    let base = temp_dir("bad-args");
    let missing = base.join("does-not-exist");
    // An invalid scope value fails as an argument error even though
    // the start directory would separately fail discovery: argument
    // validation precedes all discovery work.
    let outcome = git_resolver("fixture-app")
        .discoverer_argument(DiscovererGenus::Vcs, "scope", "bogus")
        .resolve(DirectoryClass::State, None, Some(&missing));
    match outcome {
        Err(ResolutionError::InvalidDiscovererArgument {
            discoverer,
            key,
            value,
        }) => {
            assert_eq!(discoverer, "VcsDiscovererGit");
            assert_eq!(key, "scope");
            assert_eq!(value, "bogus");
        },
        other => panic!("expected argument error, got {other:?}"),
    }
    // An unknown key is rejected even with a valid start.
    let repo = git_repo("bad-key");
    assert!(matches!(
        git_resolver("fixture-app")
            .discoverer_argument(DiscovererGenus::Vcs, "scoop", "local")
            .resolve(DirectoryClass::State, None, Some(&repo)),
        Err(ResolutionError::InvalidDiscovererArgument { .. })
    ));
    // An argument bag for an unregistered genus is a typed error that
    // names the genus.
    let outcome = Resolver::new("fixture-app")
        .expect("resolver")
        .discoverer_argument(DiscovererGenus::Devfile, "scope", "common")
        .resolve(DirectoryClass::State, None, Some(&repo));
    match outcome {
        Err(ResolutionError::InvalidDiscovererArgument {
            discoverer,
            key,
            value,
        }) => {
            assert_eq!(discoverer, "Devfile");
            assert_eq!(key, "scope");
            assert_eq!(value, "common");
        },
        other => panic!("expected genus error, got {other:?}"),
    }
}

#[test]
fn duplicate_discoverer_registration_is_refused() {
    assert!(matches!(
        git_resolver("fixture-app").register_vcs(VcsDiscovererGit),
        Err(ResolutionError::DuplicateDiscoverer(DiscovererGenus::Vcs))
    ));
    assert!(matches!(
        cargo_resolver("fixture-app").register_devfile(DevfileDiscovererCargo),
        Err(ResolutionError::DuplicateDiscoverer(
            DiscovererGenus::Devfile
        ))
    ));
}

#[test]
fn explicit_root_decides_before_arguments_or_discovery() {
    // An explicit root wins even with an invalid discoverer argument:
    // the explicit tier decided, so no discovery (and no validation)
    // runs at all.
    let base = temp_dir("explicit-first");
    let explicit = temp_dir("explicit-root");
    let resolved = git_resolver("fixture-app")
        .discoverer_argument(DiscovererGenus::Vcs, "scope", "bogus")
        .resolve(DirectoryClass::State, Some(&explicit), Some(&base))
        .expect("explicit wins");
    assert_eq!(resolved.path, explicit);
    assert_eq!(resolved.provenance, Provenance::CallerProvided);
}

/// A VCS discoverer that requires `mode = strict`, counting its
/// validations and executions so tests witness ordering.
#[derive(Debug)]
struct StrictVcs {
    validations: Arc<AtomicUsize>,
    executions: Arc<AtomicUsize>,
}

impl Discoverer for StrictVcs {
    fn genus(&self) -> DiscovererGenus {
        DiscovererGenus::Vcs
    }
}

impl VcsDiscoverer for StrictVcs {
    fn validate_args(
        &self,
        args: &BTreeMap<String, String>,
    ) -> Result<(), ResolutionError> {
        self.validations.fetch_add(1, Ordering::SeqCst);
        match args.get("mode") {
            Some(mode) if mode == "strict" => Ok(()),
            _ => Err(ResolutionError::InvalidDiscovererArgument {
                discoverer: "StrictVcs".to_string(),
                key: "mode".to_string(),
                value: args.get("mode").cloned().unwrap_or_default(),
            }),
        }
    }

    fn discover(
        &self,
        _start: &Path,
        _args: &BTreeMap<String, String>,
    ) -> Result<Option<DiscoveredBase>, ResolutionError> {
        self.executions.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }

    fn clone_box(&self) -> Box<dyn VcsDiscoverer> {
        Box::new(StrictVcs {
            validations: Arc::clone(&self.validations),
            executions: Arc::clone(&self.executions),
        })
    }
}

#[test]
fn registered_empty_bag_is_validated_before_discovery() {
    // No caller-supplied bag: the registered discoverer's effective
    // (empty) bag is still validated, so its typed refusal arrives
    // before discovery ever executes.
    let validations = Arc::new(AtomicUsize::new(0));
    let executions = Arc::new(AtomicUsize::new(0));
    let resolver = Resolver::new("fixture-app")
        .expect("resolver")
        .register_vcs(StrictVcs {
            validations: Arc::clone(&validations),
            executions: Arc::clone(&executions),
        })
        .expect("register");
    let base = temp_dir("strict-empty");
    assert!(matches!(
        resolver.resolve(DirectoryClass::State, None, Some(&base)),
        Err(ResolutionError::InvalidDiscovererArgument { .. })
    ));
    assert_eq!(validations.load(Ordering::SeqCst), 1);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
}

#[test]
fn bad_devfile_arguments_prevent_prior_vcs_execution() {
    // The VCS discoverer requires `mode = strict` but gets an
    // implicitly empty bag, and the devfile bag is explicitly invalid.
    // Validation runs in genus order before ANY discoverer executes:
    // the VCS refusal surfaces first and nothing executes.
    let validations = Arc::new(AtomicUsize::new(0));
    let executions = Arc::new(AtomicUsize::new(0));
    let resolver = Resolver::new("fixture-app")
        .expect("resolver")
        .register_vcs(StrictVcs {
            validations: Arc::clone(&validations),
            executions: Arc::clone(&executions),
        })
        .expect("register vcs")
        .register_devfile(DevfileDiscovererCargo)
        .expect("register devfile")
        .discoverer_argument(DiscovererGenus::Devfile, "bogus", "1");
    let base = temp_dir("bad-devfile-blocks");
    match resolver.resolve(DirectoryClass::State, None, Some(&base)) {
        Err(ResolutionError::InvalidDiscovererArgument {
            discoverer, ..
        }) => {
            assert_eq!(discoverer, "StrictVcs");
        },
        other => panic!("expected argument error, got {other:?}"),
    }
    assert_eq!(validations.load(Ordering::SeqCst), 1);
    assert_eq!(
        executions.load(Ordering::SeqCst),
        0,
        "vcs executed despite invalid arguments"
    );
}

#[test]
fn invalid_devfile_bag_blocks_validated_vcs_discovery() {
    // The VCS bag validates cleanly but the devfile bag is invalid:
    // the devfile refusal must surface before the validated VCS
    // discoverer executes. A mutant that runs VCS discovery right
    // after its own validation fails this control (1 execution vs 0).
    let validations = Arc::new(AtomicUsize::new(0));
    let executions = Arc::new(AtomicUsize::new(0));
    let resolver = Resolver::new("fixture-app")
        .expect("resolver")
        .register_vcs(StrictVcs {
            validations: Arc::clone(&validations),
            executions: Arc::clone(&executions),
        })
        .expect("register vcs")
        .register_devfile(DevfileDiscovererCargo)
        .expect("register devfile")
        .discoverer_argument(DiscovererGenus::Vcs, "mode", "strict")
        .discoverer_argument(DiscovererGenus::Devfile, "bogus", "1");
    let base = temp_dir("bad-devfile-after-valid-vcs");
    match resolver.resolve(DirectoryClass::State, None, Some(&base)) {
        Err(ResolutionError::InvalidDiscovererArgument {
            discoverer, ..
        }) => {
            assert_eq!(discoverer, "DevfileDiscovererCargo");
        },
        other => panic!("expected argument error, got {other:?}"),
    }
    assert_eq!(validations.load(Ordering::SeqCst), 1);
    assert_eq!(
        executions.load(Ordering::SeqCst),
        0,
        "validated vcs executed before devfile validation"
    );
}

#[test]
fn standalone_search_follows_physical_ancestry_through_dotdot() {
    // Markers at OUTER (matching) and OUTER/base/child (foreign),
    // none at OUTER/base. The start OUTER/base/child/.. physically
    // names OUTER/base, so the search must select OUTER — lexical
    // parent-stripping would select the descendant child marker.
    let outer = temp_dir("dotdot");
    fs::write(
        outer.join("Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("outer manifest");
    let child = outer.join("base/child");
    fs::create_dir_all(&child).expect("child dir");
    fs::write(
        child.join("Cargo.toml"),
        "[package]\nname = \"foreign-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("child manifest");
    let start = child.join("..");
    let resolved = cargo_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&start))
        .expect("physical ancestry");
    assert_eq!(
        resolved.path,
        start.join("..").join(".auxiliary/state/fixture-app"),
        "search must select the physical ancestor, not the lexical child"
    );
    assert_eq!(resolved.provenance, Provenance::DevelopmentWorktree);
}

#[cfg(unix)]
#[test]
fn standalone_search_refuses_dangling_marker() {
    use std::os::unix::fs::symlink;
    // The nearest marker entry is a symlink whose target is missing:
    // reading it reports NotFound, but the entry exists, so this is
    // an unreadable marker — a typed fault, never a silent walk past
    // it to the matching ancestor.
    let outer = temp_dir("dangling");
    fs::write(
        outer.join("Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("outer manifest");
    let child = outer.join("child");
    fs::create_dir_all(&child).expect("child dir");
    let link = child.join("Cargo.toml");
    symlink("missing-manifest.toml", &link).expect("dangling link");
    assert!(matches!(
        cargo_resolver("fixture-app").resolve(
            DirectoryClass::State,
            None,
            Some(&child)
        ),
        Err(ResolutionError::Unreadable(_))
    ));
    // Removing the link leaves genuine absence: the ancestor is then
    // selected normally.
    fs::remove_file(&link).expect("remove link");
    let resolved = cargo_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&child))
        .expect("genuine absence");
    assert_eq!(resolved.path, outer.join(".auxiliary/state/fixture-app"));
    assert_eq!(resolved.provenance, Provenance::DevelopmentWorktree);
}

#[cfg(unix)]
#[test]
fn standalone_search_reads_through_healthy_marker_link() {
    use std::os::unix::fs::symlink;
    // A readable symlink marker is validated by its target's content
    // but selects its own containing directory as the base: the link
    // is followed for identity, never for base location.
    let outer = temp_dir("healthy-link");
    fs::write(
        outer.join("Cargo.toml"),
        "[package]\nname = \"foreign-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("outer manifest");
    let sibling = outer.join("sibling");
    fs::create_dir_all(&sibling).expect("sibling dir");
    let target = sibling.join("Cargo.toml");
    fs::write(
        &target,
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("target manifest");
    let child = outer.join("child");
    fs::create_dir_all(&child).expect("child dir");
    symlink(&target, child.join("Cargo.toml")).expect("healthy link");
    // The foreign outer marker must not matter: the nearest readable
    // marker decides, and the base is the link's directory — neither
    // the target's directory nor any ancestor.
    let resolved = cargo_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&child))
        .expect("healthy link");
    assert_eq!(resolved.path, child.join(".auxiliary/state/fixture-app"));
    assert_eq!(resolved.provenance, Provenance::DevelopmentWorktree);
}

#[cfg(unix)]
#[test]
fn standalone_search_follows_physical_ancestry_through_symlink() {
    use std::os::unix::fs::symlink;
    // OUTER matches, OUTER/base/child carries a foreign marker, and
    // `link` is a symlink to the child. Starting at link/.. physically
    // names OUTER/base (no marker); lexical parent-stripping would
    // read the foreign child marker and report a mismatch.
    let outer = temp_dir("symlink-ancestry");
    fs::write(
        outer.join("Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("outer manifest");
    let child = outer.join("base/child");
    fs::create_dir_all(&child).expect("child dir");
    fs::write(
        child.join("Cargo.toml"),
        "[package]\nname = \"foreign-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("child manifest");
    let link = outer.join("link");
    symlink(&child, &link).expect("symlink");
    let start = link.join("..");
    let resolved = cargo_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&start))
        .expect("physical ancestry through symlink");
    assert_eq!(
        resolved.path,
        start.join("..").join(".auxiliary/state/fixture-app"),
        "search must follow the symlink to the physical ancestor"
    );
    assert_eq!(resolved.provenance, Provenance::DevelopmentWorktree);
}

/// Builds an all-filled family from four distinct absolute bases.
fn filled_family(tag: &str) -> (PartialRoots, Vec<PathBuf>) {
    let mut paths = Vec::new();
    let mut pick = |kind: &str| {
        let base = temp_dir(&format!("{tag}-{kind}"));
        paths.push(base.clone());
        Some(base)
    };
    let partial = PartialRoots {
        configuration: pick("configuration"),
        state: pick("state"),
        cache: pick("cache"),
        data: pick("data"),
    };
    (partial, paths)
}

#[test]
fn family_all_filled_validates_without_executing() {
    // StrictVcs requires `mode = strict` but gets no bag, and the
    // devfile bag is invalid: all-filled bypasses argument
    // validation, discovery, and platform entirely, so nothing
    // executes and every root reports CallerProvided.
    let validations = Arc::new(AtomicUsize::new(0));
    let executions = Arc::new(AtomicUsize::new(0));
    let resolver = Resolver::new("fixture-app")
        .expect("resolver")
        .register_vcs(StrictVcs {
            validations: Arc::clone(&validations),
            executions: Arc::clone(&executions),
        })
        .expect("register vcs")
        .register_devfile(DevfileDiscovererCargo)
        .expect("register devfile")
        .discoverer_argument(DiscovererGenus::Devfile, "bogus", "1");
    let (partial, paths) = filled_family("family-valid");
    let base = temp_dir("family-start");
    let bundle = resolver
        .resolve_partially_explicit(&partial, Some(&base))
        .expect("all filled");
    for (root, path) in [
        (&bundle.configuration, &paths[0]),
        (&bundle.state, &paths[1]),
        (&bundle.cache, &paths[2]),
        (&bundle.data, &paths[3]),
    ] {
        assert_eq!(&root.path, path);
        assert_eq!(root.provenance, Provenance::CallerProvided);
    }
    assert_eq!(validations.load(Ordering::SeqCst), 0);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
}

#[test]
fn family_gaps_fill_through_discovery() {
    // Only State supplied; the rest run the normal chain. With the
    // Cargo discoverer and a fixture start, gaps resolve through the
    // standalone upward search as DevelopmentWorktree.
    let outer = temp_dir("family-gaps");
    fs::write(
        outer.join("Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("outer manifest");
    let leaf = outer.join("nested");
    fs::create_dir_all(&leaf).expect("leaf dir");
    let state = temp_dir("family-state");
    let partial = PartialRoots {
        configuration: None,
        state: Some(state.clone()),
        cache: None,
        data: None,
    };
    let bundle = cargo_resolver("fixture-app")
        .resolve_partially_explicit(&partial, Some(&leaf))
        .expect("gaps fill");
    assert_eq!(bundle.state.path, state);
    assert_eq!(bundle.state.provenance, Provenance::CallerProvided);
    for (root, segment) in [
        (&bundle.configuration, "configuration"),
        (&bundle.cache, "cache"),
        (&bundle.data, "data"),
    ] {
        assert_eq!(
            root.path,
            outer.join(".auxiliary").join(segment).join("fixture-app"),
            "gap resolves through discovery"
        );
        assert_eq!(root.provenance, Provenance::DevelopmentWorktree);
    }
}

#[test]
fn family_gaps_without_start_fall_back_to_platform() {
    // No start and no discoverers: gaps fall back to platform roots
    // without consulting ambient state; a bad argument bag still
    // errors instead of falling through silently.
    let state = temp_dir("family-nostart");
    let partial = PartialRoots {
        configuration: None,
        state: Some(state.clone()),
        cache: None,
        data: None,
    };
    let bundle = Resolver::new("fixture-app")
        .expect("resolver")
        .resolve_partially_explicit(&partial, None)
        .expect("platform fallback");
    assert_eq!(bundle.state.path, state);
    for root in [&bundle.configuration, &bundle.cache, &bundle.data] {
        assert!(matches!(
            root.provenance,
            Provenance::PlatformEnvironment | Provenance::PlatformDefault
        ));
    }
    // A bad bag is not suppressed by the platform fallback.
    let outcome = Resolver::new("fixture-app")
        .expect("resolver")
        .register_devfile(DevfileDiscovererCargo)
        .expect("register devfile")
        .discoverer_argument(DiscovererGenus::Devfile, "bogus", "1")
        .resolve_partially_explicit(&partial, None);
    assert!(matches!(
        outcome,
        Err(ResolutionError::InvalidDiscovererArgument { .. })
    ));
}

#[test]
fn family_registered_discoverer_never_runs_without_start() {
    // A valid registered discoverer with all gaps and no start:
    // discovery must not execute (zero counter) and every kind
    // reports platform provenance. Catches an ambient-origin mutant
    // that falls back to the process working directory.
    let validations = Arc::new(AtomicUsize::new(0));
    let executions = Arc::new(AtomicUsize::new(0));
    let resolver = Resolver::new("fixture-app")
        .expect("resolver")
        .register_vcs(StrictVcs {
            validations: Arc::clone(&validations),
            executions: Arc::clone(&executions),
        })
        .expect("register vcs")
        .discoverer_argument(DiscovererGenus::Vcs, "mode", "strict");
    let bundle = resolver
        .resolve_partially_explicit(&PartialRoots::default(), None)
        .expect("platform fallback");
    assert_eq!(
        executions.load(Ordering::SeqCst),
        0,
        "discovery executed without a start directory"
    );
    assert!(
        validations.load(Ordering::SeqCst) > 0,
        "argument validation ran and passed"
    );
    for root in [
        &bundle.configuration,
        &bundle.state,
        &bundle.cache,
        &bundle.data,
    ] {
        assert!(matches!(
            root.provenance,
            Provenance::PlatformEnvironment | Provenance::PlatformDefault
        ));
    }
}

#[test]
fn family_invalid_some_fails_before_gap_filling() {
    // Configuration is an early gap while State is an invalid late
    // `Some`: the typed refusal surfaces before any gap work runs —
    // no argument validation, no discovery, no executions.
    let validations = Arc::new(AtomicUsize::new(0));
    let executions = Arc::new(AtomicUsize::new(0));
    let resolver = Resolver::new("fixture-app")
        .expect("resolver")
        .register_vcs(StrictVcs {
            validations: Arc::clone(&validations),
            executions: Arc::clone(&executions),
        })
        .expect("register vcs")
        .register_devfile(DevfileDiscovererCargo)
        .expect("register devfile");
    let base = temp_dir("family-order");
    let partial = PartialRoots {
        configuration: None,
        state: Some(PathBuf::new()),
        cache: None,
        data: None,
    };
    assert!(matches!(
        resolver.resolve_partially_explicit(&partial, Some(&base)),
        Err(ResolutionError::EmptyPath)
    ));
    assert_eq!(validations.load(Ordering::SeqCst), 0);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
}

#[test]
fn family_some_roots_are_used_literally() {
    // Base-shaped values are used exactly as given: no class or
    // structure segments are appended to a `Some` value.
    let raw_base = temp_dir("family-literal").join("litrpg");
    let partial = PartialRoots {
        configuration: Some(raw_base.clone()),
        state: Some(raw_base.clone()),
        cache: Some(raw_base.clone()),
        data: Some(raw_base.clone()),
    };
    let bundle = Resolver::new("litrpg")
        .expect("resolver")
        .resolve_partially_explicit(&partial, None)
        .expect("literal bundle");
    for root in [
        &bundle.configuration,
        &bundle.state,
        &bundle.cache,
        &bundle.data,
    ] {
        assert_eq!(root.path, raw_base);
        assert_eq!(root.provenance, Provenance::CallerProvided);
    }
}

#[test]
fn local_root_composes_exactly_and_inherits_provenance() {
    let structure = LocalStructure::default();
    // Supplied base example from the contract. The base is never
    // created: composition is pure and platform-portable.
    let supplied_base = temp_dir("supplied-base");
    let supplied = local_root(
        &approots::ResolvedRoot {
            path: supplied_base.clone(),
            provenance: Provenance::SuppliedDirectory,
        },
        DirectoryClass::State,
        "litrpg",
        &structure,
    )
    .expect("supplied base");
    assert_eq!(supplied.path, supplied_base.join(".auxiliary/state/litrpg"));
    assert_eq!(supplied.provenance, Provenance::SuppliedDirectory);
    // A raw base ending in the application name still composes
    // literally: rawness is a caller precondition, not a checked shape.
    let raw_base = temp_dir("raw-base").join("litrpg");
    let raw = local_root(
        &approots::ResolvedRoot {
            path: raw_base.clone(),
            provenance: Provenance::CallerProvided,
        },
        DirectoryClass::State,
        "litrpg",
        &structure,
    )
    .expect("raw base");
    assert_eq!(raw.path, raw_base.join(".auxiliary/state/litrpg"));
    assert_eq!(raw.provenance, Provenance::CallerProvided);
}

#[test]
fn local_root_refuses_unresolved_bases() {
    let structure = LocalStructure::default();
    let base = |path: PathBuf, provenance: Provenance| {
        approots::ResolvedRoot { path, provenance }
    };
    // Empty and relative bases are typed refusals, never propagated
    // relative roots: local_root takes no explicit start directory.
    assert!(matches!(
        local_root(
            &base(PathBuf::new(), Provenance::SuppliedDirectory),
            DirectoryClass::State,
            "fixture-app",
            &structure,
        ),
        Err(ResolutionError::EmptyPath)
    ));
    assert!(matches!(
        local_root(
            &base(
                PathBuf::from("relative/base"),
                Provenance::SuppliedDirectory,
            ),
            DirectoryClass::State,
            "fixture-app",
            &structure,
        ),
        Err(ResolutionError::RelativeStartDirectory(_))
    ));
    #[cfg(windows)]
    {
        // Windows shapes consult ambient drive state or replace the
        // base during a join.
        assert!(matches!(
            local_root(
                &base(PathBuf::from("C:foo"), Provenance::SuppliedDirectory,),
                DirectoryClass::State,
                "fixture-app",
                &structure,
            ),
            Err(ResolutionError::InvalidName(_))
        ));
        assert!(matches!(
            local_root(
                &base(PathBuf::from("\\foo"), Provenance::SuppliedDirectory,),
                DirectoryClass::State,
                "fixture-app",
                &structure,
            ),
            Err(ResolutionError::InvalidName(_))
        ));
    }
}

#[test]
fn platform_tier_never_applies_local_structure() {
    // With no discoverers, a non-repository start resolves the
    // platform root exactly once namespaced: no `.auxiliary` segment
    // is ever applied on the platform tier.
    let plain = temp_dir("platform-tier");
    for class in [
        DirectoryClass::Configuration,
        DirectoryClass::State,
        DirectoryClass::Cache,
        DirectoryClass::Data,
    ] {
        let resolved = Resolver::new("fixture-app")
            .expect("resolver")
            .resolve(class, None, Some(&plain))
            .expect("platform tier");
        assert_eq!(
            resolved.path.file_name().expect("file name"),
            "fixture-app"
        );
        assert!(
            !resolved
                .path
                .components()
                .any(|component| component.as_os_str() == ".auxiliary")
        );
    }
}

#[test]
fn discovery_ignores_process_working_directory() {
    // Child mode: resolve a fixed start under a foreign cwd.
    if let Ok(start) = std::env::var("APPROOTS_CWD_CHILD") {
        let start = PathBuf::from(&start);
        let expected =
            PathBuf::from(std::env::var("APPROOTS_CWD_EXPECT").expect("base"));
        let resolved = git_resolver("fixture-app")
            .resolve(DirectoryClass::State, None, Some(&start))
            .expect("child resolve");
        assert_eq!(
            resolved.path,
            expected.join(".auxiliary/state/fixture-app")
        );
        return;
    }
    let repo = git_repo("cwd-proof");
    let exe = std::env::current_exe().expect("test exe");
    for cwd in [temp_dir("cwd-a"), temp_dir("cwd-b")] {
        let output = Command::new(&exe)
            .args([
                "discovery_ignores_process_working_directory",
                "--exact",
                "--nocapture",
            ])
            .env("APPROOTS_CWD_CHILD", &repo)
            .env("APPROOTS_CWD_EXPECT", &repo)
            .current_dir(&cwd)
            .output()
            .expect("spawn cwd child");
        assert!(
            output.status.success(),
            "cwd child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
