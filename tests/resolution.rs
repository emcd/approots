//! Behavioral tests through the public resolution contract.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use approots::development::{DevelopmentLayout, development_root};
use approots::platform::sanitize_name;
use approots::{
    DevelopmentScope, DirectoryKind, Provenance, ResolutionError, Resolver,
    discover_checkout, supplied_root, validate_cargo_package, validate_token,
};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir()
        .join(format!("approots-{tag}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&path).expect("create temp dir");
    path.canonicalize().expect("canonical temp dir")
}

fn git(dir: &Path, args: &[&str]) {
    // Blast-by-prefix: commit hooks export GIT_INDEX_FILE and friends;
    // fixtures must not inherit any GIT_* routing variable.
    let mut command = Command::new("git");
    command.args(args).current_dir(dir);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(&key);
        }
    }
    command.env("GIT_CONFIG_NOSYSTEM", "1");
    let output = command.output().expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_repo(tag: &str) -> PathBuf {
    let repo = temp_dir(tag);
    git(&repo, &["init", "-q", "-b", "master"]);
    git(&repo, &["config", "user.email", "test@example.invalid"]);
    git(&repo, &["config", "user.name", "Test"]);
    fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "init"]);
    repo
}

#[test]
fn supplied_resolution_uses_absolute_base_only() {
    let base = temp_dir("supplied");
    let resolved = supplied_root(&base, DirectoryKind::Configuration)
        .expect("resolve supplied");
    assert_eq!(resolved.path, base.join("configuration"));
    assert_eq!(resolved.provenance, Provenance::SuppliedDirectory);
    assert!(!resolved.path.exists(), "resolution creates nothing");
    assert!(matches!(
        supplied_root(Path::new("relative"), DirectoryKind::State),
        Err(ResolutionError::RelativeStartDirectory(_))
    ));
    assert!(matches!(
        supplied_root(Path::new(""), DirectoryKind::State),
        Err(ResolutionError::EmptyPath)
    ));
}

#[test]
fn normal_clone_collapses_roots_from_nested_directory() {
    let repo = git_repo("clone");
    let nested = repo.join("a/b");
    fs::create_dir_all(&nested).expect("nested dirs");
    let checkout = discover_checkout(&nested).expect("discover");
    assert_eq!(checkout.worktree_root, repo);
    assert_eq!(checkout.common_worktree_root, repo);
    // Regression: relative --git-common-dir resolves against the
    // invocation directory, never the worktree root.
    assert_eq!(checkout.git_common_dir, repo.join(".git"));
}

#[test]
fn linked_worktree_distinguishes_roots() {
    let repo = git_repo("linked-main");
    let linked = temp_dir("linked-wt").join("wt");
    let linked_arg = linked.to_string_lossy().to_string();
    git(&repo, &["worktree", "add", "-q", &linked_arg]);
    let checkout = discover_checkout(&linked).expect("discover linked");
    assert_eq!(
        checkout.worktree_root,
        linked.canonicalize().expect("canonical linked")
    );
    assert_eq!(checkout.common_worktree_root, repo);
    assert_ne!(checkout.worktree_root, checkout.common_worktree_root);
}

#[test]
fn identity_validation_accepts_rejects_and_bounds() {
    let repo = git_repo("identity");
    let checkout = discover_checkout(&repo).expect("discover");
    validate_cargo_package(&checkout, "fixture-app").expect("match");
    assert!(matches!(
        validate_cargo_package(&checkout, "other-app"),
        Err(ResolutionError::IdentityMismatch { .. })
    ));
    fs::write(repo.join("Cargo.toml"), "[workspace]\nmembers = []\n")
        .expect("virtual manifest");
    assert!(matches!(
        validate_cargo_package(&checkout, "fixture-app"),
        Err(ResolutionError::VirtualWorkspace(_))
    ));
}

#[test]
fn development_scopes_and_layout_escape_refusal() {
    let repo = git_repo("devlayout");
    let checkout = discover_checkout(&repo).expect("discover");
    let layout = DevelopmentLayout::default();
    let common = development_root(
        &checkout,
        DevelopmentScope::Common,
        DirectoryKind::State,
        "fixture-app",
        &layout,
    )
    .expect("common root");
    assert_eq!(common.path, repo.join(".auxiliary/state/fixture-app"));
    assert_eq!(common.provenance, Provenance::DevelopmentCommon);
    assert!(!common.path.exists(), "resolution creates nothing");

    let bad = DevelopmentLayout {
        auxiliary: PathBuf::from("../escape"),
        ..DevelopmentLayout::default()
    };
    assert!(
        development_root(
            &checkout,
            DevelopmentScope::Worktree,
            DirectoryKind::State,
            "fixture-app",
            &bad,
        )
        .is_err()
    );
    assert!(validate_token("a/b").is_err());
    assert!(validate_token("..").is_err());
    assert!(validate_token("good-name_1").is_ok());
}

#[test]
fn precedence_prefers_explicit_then_platform_when_dev_disabled() {
    let repo = git_repo("precedence");
    let explicit = temp_dir("explicit");
    let resolver = Resolver::new("fixture-app").expect("resolver");
    let resolved = resolver
        .resolve(
            DirectoryKind::State,
            Some(&explicit),
            Some(&repo),
            DevelopmentScope::Worktree,
            None,
        )
        .expect("explicit wins");
    assert_eq!(resolved.path, explicit);
    assert_eq!(resolved.provenance, Provenance::CallerProvided);

    // Development disabled: no checkout adoption, platform fallback.
    let platform = resolver
        .resolve(
            DirectoryKind::Cache,
            None,
            Some(&repo),
            DevelopmentScope::Worktree,
            None,
        )
        .expect("platform fallback");
    assert!(matches!(
        platform.provenance,
        Provenance::PlatformEnvironment | Provenance::PlatformDefault
    ));
}

#[test]
fn clone_under_modules_directory_still_discovers() {
    let parent = temp_dir("modparent");
    let repo = parent.join("modules/proj");
    fs::create_dir_all(&repo).expect("modules parent");
    git(&repo, &["init", "-q", "-b", "master"]);
    git(&repo, &["config", "user.email", "test@example.invalid"]);
    git(&repo, &["config", "user.name", "Test"]);
    fs::write(repo.join("f.txt"), "x").expect("seed");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "init"]);
    let checkout = discover_checkout(&repo).expect("discover modules");
    assert_eq!(checkout.worktree_root, repo);
}

#[test]
fn corrupt_repository_is_not_a_repository_by_git_output() {
    // Pinned decision: Git reports a corrupt HEAD identically to a
    // missing repository, so discovery classifies it NotRepository
    // (and the precedence chain falls through to the platform tier)
    // rather than re-implementing Git discovery with a .git probe.
    let repo = git_repo("corrupt");
    fs::write(repo.join(".git/HEAD"), "garbage\n").expect("corrupt");
    assert!(matches!(
        discover_checkout(&repo),
        Err(ResolutionError::NotRepository(_))
    ));
}

#[test]
fn bare_repository_is_typed_unsupported() {
    let bare = temp_dir("bare");
    git(&bare, &["init", "-q", "--bare", "-b", "master"]);
    assert!(matches!(
        discover_checkout(&bare),
        Err(ResolutionError::UnsupportedTopology(_))
    ));
}

#[test]
fn separate_git_directory_uses_real_worktree() {
    let base = temp_dir("separate");
    let git_dir = base.join("sepdir/.git");
    let worktree = base.join("wt");
    fs::create_dir_all(&git_dir).expect("git dir parent");
    fs::create_dir_all(&worktree).expect("worktree parent");
    let separate = format!("--separate-git-dir={}", git_dir.display());
    git(&base, &["init", "-q", "-b", "master", &separate, "wt"]);
    git(&worktree, &["config", "user.email", "test@example.invalid"]);
    git(&worktree, &["config", "user.name", "Test"]);
    fs::write(worktree.join("f.txt"), "x").expect("seed");
    git(&worktree, &["add", "."]);
    git(&worktree, &["commit", "-qm", "init"]);
    let checkout = discover_checkout(&worktree).expect("discover");
    assert_eq!(checkout.worktree_root, worktree);
    assert_eq!(checkout.common_worktree_root, worktree);
}

#[test]
fn clones_from_clones_and_nested_linked_invocation() {
    let repo = git_repo("clone-source");
    let clone_parent = temp_dir("clone-dest");
    let clone_path = clone_parent.join("clone2");
    let source = repo.to_string_lossy().to_string();
    let dest = clone_path.to_string_lossy().to_string();
    git(&clone_parent, &["clone", "-q", &source, &dest]);
    let checkout = discover_checkout(&clone_path).expect("clone");
    assert_eq!(checkout.worktree_root, clone_path);
    assert_eq!(checkout.common_worktree_root, clone_path);

    let linked = temp_dir("linked-nested").join("wt");
    let linked_arg = linked.to_string_lossy().to_string();
    git(&repo, &["worktree", "add", "-q", &linked_arg]);
    let nested = linked.join("deep/nest");
    fs::create_dir_all(&nested).expect("nested linked dirs");
    let linked_checkout = discover_checkout(&nested).expect("nested");
    assert_eq!(linked_checkout.common_worktree_root, repo);
    assert_ne!(
        linked_checkout.worktree_root,
        linked_checkout.common_worktree_root
    );
}

#[test]
fn layers_are_closed_ordered_and_ambiguity_is_typed() {
    use approots::LayeredRoots;

    assert!(LayeredRoots::new(Vec::new()).is_err());
    assert!(LayeredRoots::new([PathBuf::from("relative/layer")]).is_err());
    let first = temp_dir("layer-first");
    let absent = temp_dir("layer-absent-parent").join("missing");
    let roots =
        LayeredRoots::new([first.clone(), absent.clone()]).expect("roots");
    let resolved = roots.resolve_roots().expect("resolve roots");
    assert_eq!(resolved.len(), 2, "absent layer retained as candidate");
    assert_eq!(resolved[0].path, first);
    assert_eq!(resolved[1].path, absent);
    assert!(
        resolved
            .iter()
            .all(|root| root.provenance == Provenance::CallerProvided)
    );

    #[cfg(unix)]
    {
        let target = temp_dir("layer-target");
        let layer_a = temp_dir("layer-a").join("link");
        let layer_b = temp_dir("layer-b").join("link");
        std::os::unix::fs::symlink(&target, &layer_a).expect("link a");
        std::os::unix::fs::symlink(&target, &layer_b).expect("link b");
        let collapsed = LayeredRoots::new([layer_a, layer_b]).expect("roots");
        assert!(matches!(
            collapsed.resolve_roots(),
            Err(ResolutionError::Ambiguous(_))
        ));
    }
}

#[test]
fn explicit_dotdot_is_preserved_and_platform_validates() {
    let base = temp_dir("dotdot");
    let resolver = Resolver::new("fixture-app").expect("resolver");
    let explicit = PathBuf::from("alias/../state");
    let resolved = resolver
        .resolve(
            DirectoryKind::State,
            Some(&explicit),
            Some(&base),
            DevelopmentScope::Worktree,
            None,
        )
        .expect("explicit with dotdot");
    assert_eq!(resolved.path, base.join("alias/../state"));
    assert!(resolved.path.is_absolute());
    assert!(approots::platform_root("a/b", DirectoryKind::State).is_err());
    assert!(approots::platform_root("My App", DirectoryKind::Cache).is_ok());

    let dev = Resolver::new("fixture-app")
        .expect("resolver")
        .development_mode(true);
    let missing = base.join("does-not-exist");
    assert!(
        dev.resolve(
            DirectoryKind::State,
            None,
            Some(&missing),
            DevelopmentScope::Worktree,
            None,
        )
        .is_err()
    );
}

#[test]
fn multiline_decoy_manifest_does_not_spoof_identity() {
    let repo = git_repo("decoy");
    fs::write(
        repo.join("Cargo.toml"),
        "[package]\ndescription = \"\"\"\n[package]\nname = \"trusted\"\n\"\"\"\nname = \"foreign\"\nversion = \"0.1.0\"\n",
    )
    .expect("decoy manifest");
    let checkout = discover_checkout(&repo).expect("discover");
    assert!(matches!(
        validate_cargo_package(&checkout, "trusted"),
        Err(ResolutionError::IdentityMismatch { .. })
    ));
    validate_cargo_package(&checkout, "foreign").expect("real name");
}

#[test]
fn source_export_falls_through_to_platform() {
    let export = temp_dir("export");
    let resolver = Resolver::new("fixture-app")
        .expect("resolver")
        .development_mode(true);
    let resolved = resolver
        .resolve(
            DirectoryKind::State,
            None,
            Some(&export),
            DevelopmentScope::Worktree,
            None,
        )
        .expect("platform fallback for source export");
    assert!(matches!(
        resolved.provenance,
        Provenance::PlatformEnvironment | Provenance::PlatformDefault
    ));
    // Windows path shapes are refused by the token grammar everywhere.
    assert!(validate_token("a:b").is_err());
    assert!(validate_token("file:stream").is_err());
}

#[test]
fn sanitize_is_shared_and_lossy_by_design() {
    assert_eq!(sanitize_name("my app.v2"), "my-app-v2");
    assert_eq!(sanitize_name("a.b"), sanitize_name("a-b"));
}

#[cfg(unix)]
#[test]
fn symlinked_alias_discovers_real_checkout() {
    let repo = git_repo("alias-repo");
    let alias = temp_dir("alias-parent").join("alias");
    std::os::unix::fs::symlink(&repo, &alias).expect("alias link");
    let checkout = discover_checkout(&alias).expect("discover alias");
    assert_eq!(checkout.worktree_root, repo);
    assert_eq!(checkout.git_common_dir, repo.join(".git"));
    assert_eq!(checkout.common_worktree_root, repo);
}

#[test]
fn trailing_space_checkout_name_is_preserved() {
    let parent = temp_dir("space-parent");
    let repo = parent.join("repo ");
    fs::create_dir_all(&repo).expect("spaced repo");
    git(&repo, &["init", "-q", "-b", "master"]);
    git(&repo, &["config", "user.email", "test@example.invalid"]);
    git(&repo, &["config", "user.name", "Test"]);
    fs::write(repo.join("f.txt"), "x").expect("seed");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "init"]);
    let checkout = discover_checkout(&repo).expect("discover spaced");
    assert_eq!(checkout.worktree_root, repo);
}

#[test]
fn xdg_provenance_comes_from_one_snapshot() {
    // Child mode: run the assertions under the spawned environment.
    if std::env::var_os("APPROOTS_XDG_CHILD").is_some() {
        let resolved =
            approots::platform_root("fixture-app", DirectoryKind::Cache)
                .expect("child platform root");
        let expected_base = PathBuf::from(
            std::env::var("APPROOTS_EXPECT_BASE").expect("expected base"),
        );
        assert_eq!(resolved.path, expected_base.join("fixture-app"));
        let expected_provenance = match std::env::var("APPROOTS_EXPECT_PROV")
            .expect("expected provenance")
            .as_str()
        {
            "env" => Provenance::PlatformEnvironment,
            _ => Provenance::PlatformDefault,
        };
        assert_eq!(resolved.provenance, expected_provenance);
        return;
    }
    let home = temp_dir("xdg-home");
    let override_dir = temp_dir("xdg-override");
    let exe = std::env::current_exe().expect("test exe");
    for (xdg_value, expected_base, expected_provenance) in [
        (
            override_dir.to_string_lossy().into_owned(),
            override_dir.clone(),
            "env",
        ),
        ("relative/cache".to_string(), home.join(".cache"), "default"),
    ] {
        let output = Command::new(&exe)
            .args([
                "xdg_provenance_comes_from_one_snapshot",
                "--exact",
                "--nocapture",
            ])
            .env("APPROOTS_XDG_CHILD", "1")
            .env("APPROOTS_EXPECT_BASE", &expected_base)
            .env("APPROOTS_EXPECT_PROV", expected_provenance)
            .env("XDG_CACHE_HOME", xdg_value)
            .env("HOME", &home)
            .output()
            .expect("spawn xdg child");
        assert!(
            output.status.success(),
            "xdg child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
