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
    // Commit hooks export GIT_INDEX_FILE and friends; fixtures must
    // not inherit them.
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("run git");
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
fn sanitize_is_shared_and_lossy_by_design() {
    assert_eq!(sanitize_name("my app.v2"), "my-app-v2");
    assert_eq!(sanitize_name("a.b"), sanitize_name("a-b"));
}
