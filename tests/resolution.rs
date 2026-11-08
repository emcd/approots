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
    strip_verbatim_prefix(path.canonicalize().expect("canonical temp dir"))
}

/// Removes the Windows verbatim prefix (`\\?\`) that canonicalization
/// adds, since Git cannot create worktrees beneath verbatim paths and
/// reports its own results without the prefix.
fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path
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
    // invocation directory, never the worktree root. The returned
    // path keeps its semantic form; canonical comparison reconciles.
    assert_eq!(
        fs::canonicalize(&checkout.git_common_dir).expect("canonical"),
        repo.join(".git")
    );
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
    assert!(matches!(
        dev.resolve(
            DirectoryKind::State,
            None,
            Some(&missing),
            DevelopmentScope::Worktree,
            None,
        ),
        Err(ResolutionError::GitFailed(_))
    ));
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
    // The returned common dir keeps its semantic (alias) form and
    // names the real .git under canonical comparison.
    assert_eq!(
        fs::canonicalize(&checkout.git_common_dir).expect("canonical"),
        repo.join(".git")
    );
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

#[cfg(target_os = "linux")]
#[test]
fn xdg_provenance_comes_from_one_snapshot() {
    // Child mode: run the assertions under the spawned environment.
    if std::env::var("APPROOTS_XDG_CHILD").as_deref() == Ok("unavailable") {
        assert!(matches!(
            approots::platform_root("fixture-app", DirectoryKind::Cache),
            Err(ResolutionError::Unavailable(_))
        ));
        return;
    }
    if std::env::var_os("APPROOTS_XDG_CHILD").is_some() {
        let resolved =
            approots::platform_root("fixture-app", DirectoryKind::Cache)
                .expect("child platform root");
        let expected_base = PathBuf::from(
            std::env::var_os("APPROOTS_EXPECT_BASE").expect("expected base"),
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
    #[cfg(unix)]
    {
        // Native-byte XDG root survives without lossy conversion.
        use std::os::unix::ffi::OsStringExt;
        let raw_root = std::ffi::OsString::from_vec(
            override_dir
                .as_os_str()
                .as_encoded_bytes()
                .iter()
                .copied()
                .chain([0xFF])
                .collect::<Vec<u8>>(),
        );
        let exe = std::env::current_exe().expect("test exe");
        let output = Command::new(&exe)
            .args([
                "xdg_provenance_comes_from_one_snapshot",
                "--exact",
                "--nocapture",
            ])
            .env("APPROOTS_XDG_CHILD", "1")
            .env("APPROOTS_EXPECT_BASE", PathBuf::from(raw_root.clone()))
            .env("APPROOTS_EXPECT_PROV", "env")
            .env("XDG_CACHE_HOME", raw_root)
            .env("HOME", &home)
            .output()
            .expect("spawn nonutf8 xdg child");
        assert!(
            output.status.success(),
            "nonutf8 xdg child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        // A relative HOME yields typed Unavailable, never a
        // propagated relative platform root.
        let output = Command::new(&exe)
            .args([
                "xdg_provenance_comes_from_one_snapshot",
                "--exact",
                "--nocapture",
            ])
            .env("APPROOTS_XDG_CHILD", "unavailable")
            .env_remove("XDG_CACHE_HOME")
            .env("HOME", "relative-home")
            .output()
            .expect("spawn relative-home child");
        assert!(
            output.status.success(),
            "relative-home child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
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

#[cfg(unix)]
#[test]
fn symlink_dotdot_start_keeps_semantic_common_dir() {
    // alias -> repo/sub; starting at alias/.. names the repo root to
    // the OS but lexically pops to the alias parent, which is not a
    // checkout at all. The returned common dir must keep the semantic
    // alias/.. form (lexical popping would retarget it).
    let repo = git_repo("symlink-dd");
    fs::create_dir_all(repo.join("sub")).expect("sub dir");
    let parent = temp_dir("symlink-dd-parent");
    let alias = parent.join("alias");
    std::os::unix::fs::symlink(repo.join("sub"), &alias).expect("alias");
    let start = alias.join("..");
    let checkout = discover_checkout(&start).expect("discover");
    assert_eq!(checkout.worktree_root, repo);
    assert_eq!(checkout.git_common_dir, alias.join("../.git"));
    assert_eq!(
        fs::canonicalize(&checkout.git_common_dir).expect("canonical"),
        repo.join(".git")
    );
}

#[cfg(unix)]
#[test]
fn exotic_checkout_names_survive_discovery() {
    #[cfg(target_os = "linux")]
    use std::os::unix::ffi::OsStringExt;

    let parent = temp_dir("exotic");
    #[allow(unused_mut)]
    let mut names = vec![
        std::ffi::OsString::from("sp ace"),
        std::ffi::OsString::from("trail "),
        std::ffi::OsString::from("new\nline"),
        std::ffi::OsString::from("carriage\r"),
    ];
    // macOS filesystems reject non-UTF8 names (EILSEQ); Linux allows them.
    #[cfg(target_os = "linux")]
    names.push(std::ffi::OsString::from_vec(b"nonutf8-\xff".to_vec()));
    for name in names {
        let repo = parent.join(&name);
        fs::create_dir_all(&repo).expect("exotic repo");
        git(&repo, &["init", "-q", "-b", "master"]);
        let checkout = discover_checkout(&repo).expect("discover exotic");
        assert_eq!(checkout.worktree_root, repo);
    }
    // A symlinked alias containing .. still names the real checkout.
    let repo = git_repo("alias-dotdot");
    fs::create_dir_all(repo.join("a")).expect("dotdot intermediate");
    let alias_parent = temp_dir("alias-dd-parent");
    let alias = alias_parent.join("alias");
    std::os::unix::fs::symlink(&repo, &alias).expect("alias link");
    let start = alias.join("a/../");
    let checkout = discover_checkout(&start).expect("alias dotdot");
    assert_eq!(checkout.worktree_root, repo);
    assert_eq!(
        fs::canonicalize(&checkout.git_common_dir).expect("canonical"),
        repo.join(".git")
    );
}
