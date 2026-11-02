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
    let first = temp_dir("layer-first");
    let second = temp_dir("layer-second");
    fs::write(second.join("app.toml"), "second").expect("second file");
    let roots =
        LayeredRoots::new([first.clone(), second.clone()]).expect("roots");
    let found = roots
        .resolve_file(Path::new("app.toml"))
        .expect("lookup")
        .expect("found in second layer");
    assert_eq!(found, second.join("app.toml"));

    fs::write(first.join("app.toml"), "first").expect("first file");
    let found = roots
        .resolve_file(Path::new("app.toml"))
        .expect("lookup")
        .expect("found in first layer");
    assert_eq!(found, first.join("app.toml"));

    #[cfg(unix)]
    {
        let target = temp_dir("layer-target");
        let shared = target.join("shared.toml");
        fs::write(&shared, "shared").expect("shared file");
        let layer_a = temp_dir("layer-a");
        let layer_b = temp_dir("layer-b");
        std::os::unix::fs::symlink(&shared, layer_a.join("app.toml"))
            .expect("symlink a");
        std::os::unix::fs::symlink(&shared, layer_b.join("app.toml"))
            .expect("symlink b");
        let collapsed = LayeredRoots::new([layer_a, layer_b]).expect("roots");
        assert!(matches!(
            collapsed.resolve_file(Path::new("app.toml")),
            Err(ResolutionError::Ambiguous(_))
        ));
    }
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
