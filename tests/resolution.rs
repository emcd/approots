//! Behavioral tests through the public resolution contract.

mod common;

use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::process::Command;

use approots::platform::sanitize_name;
use approots::{
    DirectoryClass, Provenance, ResolutionError, Resolver, VcsDiscovererGit,
    supplied_root, validate_token,
};
use common::*;

#[test]
fn supplied_resolution_uses_absolute_base_only() {
    let base = temp_dir("supplied");
    let resolved = supplied_root(&base, DirectoryClass::Configuration)
        .expect("resolve supplied");
    assert_eq!(resolved.path, base.join("configuration"));
    assert_eq!(resolved.provenance, Provenance::SuppliedDirectory);
    assert!(!resolved.path.exists(), "resolution creates nothing");
    assert!(matches!(
        supplied_root(Path::new("relative"), DirectoryClass::State),
        Err(ResolutionError::RelativeStartDirectory(_))
    ));
    assert!(matches!(
        supplied_root(Path::new(""), DirectoryClass::State),
        Err(ResolutionError::EmptyPath)
    ));
}

#[test]
fn normal_clone_collapses_scopes_from_nested_directory() {
    let repo = git_repo("clone");
    let nested = repo.join("a/b");
    fs::create_dir_all(&nested).expect("nested dirs");
    // Nested invocation resolves against the invocation directory:
    // both scopes collapse to the same checkout base.
    for (scope, provenance) in [
        (
            VcsDiscovererGit::SCOPE_LOCAL,
            Provenance::DevelopmentWorktree,
        ),
        (
            VcsDiscovererGit::SCOPE_COMMON,
            Provenance::DevelopmentCommon,
        ),
    ] {
        let resolved = scope_arg(git_resolver("fixture-app"), scope)
            .resolve(DirectoryClass::State, None, Some(&nested))
            .expect("discover nested");
        assert_eq!(resolved.path, repo.join(".auxiliary/state/fixture-app"));
        assert_eq!(resolved.provenance, provenance);
    }
}

#[test]
fn linked_worktree_scopes_select_distinct_roots() {
    let repo = git_repo("linked-main");
    let linked = temp_dir("linked-wt").join("wt");
    let linked_arg = linked.to_string_lossy().to_string();
    git(&repo, &["worktree", "add", "-q", &linked_arg]);
    let expected_linked =
        strip_verbatim_prefix(linked.canonicalize().expect("canonical"));
    let local =
        scope_arg(git_resolver("fixture-app"), VcsDiscovererGit::SCOPE_LOCAL)
            .resolve(DirectoryClass::State, None, Some(&linked))
            .expect("discover linked");
    assert_eq!(
        local.path,
        expected_linked.join(".auxiliary/state/fixture-app")
    );
    assert_eq!(local.provenance, Provenance::DevelopmentWorktree);
    let common =
        scope_arg(git_resolver("fixture-app"), VcsDiscovererGit::SCOPE_COMMON)
            .resolve(DirectoryClass::Configuration, None, Some(&linked))
            .expect("discover common");
    assert_eq!(
        common.path,
        repo.join(".auxiliary/configuration/fixture-app")
    );
    assert_eq!(common.provenance, Provenance::DevelopmentCommon);
    assert_ne!(local.path, common.path);
}

#[test]
fn precedence_prefers_explicit_then_platform_without_discoverers() {
    let repo = git_repo("precedence");
    let explicit = temp_dir("explicit");
    let resolver = Resolver::new("fixture-app").expect("resolver");
    let resolved = resolver
        .resolve(DirectoryClass::State, Some(&explicit), Some(&repo))
        .expect("explicit wins");
    assert_eq!(resolved.path, explicit);
    assert_eq!(resolved.provenance, Provenance::CallerProvided);

    // No discoverer registered: no checkout adoption, platform fallback.
    let platform = resolver
        .resolve(DirectoryClass::Cache, None, Some(&repo))
        .expect("platform fallback");
    assert!(matches!(
        platform.provenance,
        Provenance::PlatformEnvironment | Provenance::PlatformDefault
    ));
    assert!(
        !platform.path.join(".auxiliary").exists()
            || !platform.path.starts_with(&repo)
    );
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
    let resolved = git_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&repo))
        .expect("discover modules");
    assert_eq!(resolved.path, repo.join(".auxiliary/state/fixture-app"));
    assert_eq!(resolved.provenance, Provenance::DevelopmentWorktree);
}

#[test]
fn corrupt_repository_falls_through_by_git_output() {
    // Pinned decision: Git reports a corrupt HEAD identically to a
    // missing repository, so discovery classifies it NotRepository
    // (and the precedence chain falls through to the platform tier)
    // rather than re-implementing Git discovery with a .git probe.
    let repo = git_repo("corrupt");
    fs::write(repo.join(".git/HEAD"), "garbage\n").expect("corrupt");
    let resolved = git_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&repo))
        .expect("corrupt falls through");
    assert!(matches!(
        resolved.provenance,
        Provenance::PlatformEnvironment | Provenance::PlatformDefault
    ));
}

#[test]
fn bare_repository_is_typed_unsupported() {
    let bare = temp_dir("bare");
    git(&bare, &["init", "-q", "--bare", "-b", "master"]);
    assert!(matches!(
        git_cargo_resolver("fixture-app").resolve(
            DirectoryClass::State,
            None,
            Some(&bare)
        ),
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
    let resolved = git_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&worktree))
        .expect("discover");
    assert_eq!(resolved.path, worktree.join(".auxiliary/state/fixture-app"));
    assert_eq!(resolved.provenance, Provenance::DevelopmentWorktree);
}

#[test]
fn clones_from_clones_and_nested_linked_invocation() {
    let repo = git_repo("clone-source");
    let clone_parent = temp_dir("clone-dest");
    let clone_path = clone_parent.join("clone2");
    let source = repo.to_string_lossy().to_string();
    let dest = clone_path.to_string_lossy().to_string();
    git(&clone_parent, &["clone", "-q", &source, &dest]);
    for scope in [
        VcsDiscovererGit::SCOPE_LOCAL,
        VcsDiscovererGit::SCOPE_COMMON,
    ] {
        let resolved = scope_arg(git_resolver("fixture-app"), scope)
            .resolve(DirectoryClass::State, None, Some(&clone_path))
            .expect("clone");
        assert_eq!(
            resolved.path,
            clone_path.join(".auxiliary/state/fixture-app")
        );
    }

    let linked = temp_dir("linked-nested").join("wt");
    let linked_arg = linked.to_string_lossy().to_string();
    git(&repo, &["worktree", "add", "-q", &linked_arg]);
    let nested = linked.join("deep/nest");
    fs::create_dir_all(&nested).expect("nested linked dirs");
    let local =
        scope_arg(git_resolver("fixture-app"), VcsDiscovererGit::SCOPE_LOCAL)
            .resolve(DirectoryClass::State, None, Some(&nested))
            .expect("nested local");
    let expected_linked =
        strip_verbatim_prefix(linked.canonicalize().expect("canonical"));
    assert_eq!(
        local.path,
        expected_linked.join(".auxiliary/state/fixture-app")
    );
    let common =
        scope_arg(git_resolver("fixture-app"), VcsDiscovererGit::SCOPE_COMMON)
            .resolve(DirectoryClass::State, None, Some(&nested))
            .expect("nested common");
    assert_eq!(common.path, repo.join(".auxiliary/state/fixture-app"));
    assert_ne!(local.path, common.path);
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
        .resolve(DirectoryClass::State, Some(&explicit), Some(&base))
        .expect("explicit with dotdot");
    assert_eq!(resolved.path, base.join("alias/../state"));
    assert!(resolved.path.is_absolute());
    assert!(approots::platform_root("a/b", DirectoryClass::State).is_err());
    assert!(approots::platform_root("My App", DirectoryClass::Cache).is_ok());

    // A missing start directory is a typed Git failure, not a silent
    // platform fallback and not a missing executable.
    let missing = base.join("does-not-exist");
    assert!(matches!(
        git_resolver("fixture-app").resolve(
            DirectoryClass::State,
            None,
            Some(&missing),
        ),
        Err(ResolutionError::GitFailed(_))
    ));
}

#[test]
fn platform_roots_resolve_for_every_class() {
    // Every class resolves on every supported platform: the state class
    // bases on local application data (Windows) or application support
    // (macOS) where the provider has no state directory.
    for class in [
        DirectoryClass::Configuration,
        DirectoryClass::State,
        DirectoryClass::Cache,
        DirectoryClass::Data,
    ] {
        let resolved = approots::platform_root("fixture-app", class)
            .expect("platform root");
        assert!(resolved.path.is_absolute(), "{class:?} root not absolute");
        assert_eq!(
            resolved.path.file_name().expect("file name"),
            "fixture-app",
            "{class:?} root does not end in the application name"
        );
        assert!(matches!(
            resolved.provenance,
            Provenance::PlatformEnvironment | Provenance::PlatformDefault
        ));
    }
}

#[test]
fn distinct_directory_classes_append_only_on_aliased_platforms() {
    let plain = Resolver::new("fixture-app").expect("resolver");
    let distinct = Resolver::new("fixture-app")
        .expect("resolver")
        .distinct_directory_classes(true);
    for class in [
        DirectoryClass::Configuration,
        DirectoryClass::State,
        DirectoryClass::Cache,
        DirectoryClass::Data,
    ] {
        let standard =
            plain.resolve(class, None, None).expect("standard root");
        let separated =
            distinct.resolve(class, None, None).expect("distinct root");
        assert_eq!(standard.provenance, separated.provenance);
        if cfg!(any(windows, target_os = "macos")) {
            assert_eq!(separated.path, standard.path.join(class.segment()));
        } else {
            assert_eq!(separated.path, standard.path);
        }
    }
}

#[cfg(windows)]
#[test]
fn windows_state_root_uses_local_application_data() {
    let dirs = directories::BaseDirs::new().expect("base dirs");
    let resolved =
        approots::platform_root("fixture-app", DirectoryClass::State)
            .expect("state root");
    assert_eq!(resolved.path, dirs.data_local_dir().join("fixture-app"));
    assert_eq!(resolved.provenance, Provenance::PlatformDefault);
}

#[cfg(target_os = "macos")]
#[test]
fn macos_state_root_uses_application_support() {
    let dirs = directories::BaseDirs::new().expect("base dirs");
    let resolved =
        approots::platform_root("fixture-app", DirectoryClass::State)
            .expect("state root");
    assert_eq!(resolved.path, dirs.data_dir().join("fixture-app"));
    assert_eq!(resolved.provenance, Provenance::PlatformDefault);
}

#[cfg(windows)]
#[test]
fn windows_drive_relative_and_root_relative_roots_are_refused() {
    let resolver = Resolver::new("fixture-app").expect("resolver");
    let base = temp_dir("win-shape");
    // Drive-relative (C:foo) consults ambient drive state.
    assert!(matches!(
        resolver.resolve(
            DirectoryClass::State,
            Some(Path::new("C:foo")),
            Some(&base),
        ),
        Err(ResolutionError::InvalidName(_))
    ));
    // Root-relative (\foo) replaces the base during a join.
    assert!(matches!(
        resolver.resolve(
            DirectoryClass::State,
            Some(Path::new("\\foo")),
            Some(&base),
        ),
        Err(ResolutionError::InvalidName(_))
    ));
}

#[cfg(windows)]
#[test]
fn windows_linked_checkout_discovers_through_alias() {
    // A usable alias is mandatory on the verification runner: try a
    // directory symlink, then a junction, and fail with both sets of
    // diagnostics rather than silently skipping the alias assertions.
    let repo = git_repo("junction-target");
    let link_parent = temp_dir("junction-parent");
    let link = link_parent.join("link");
    if let Err(symlink_error) = std::os::windows::fs::symlink_dir(&repo, &link)
    {
        let output = Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &link.to_string_lossy(),
                &repo.to_string_lossy(),
            ])
            .output()
            .expect("run mklink");
        assert!(
            output.status.success(),
            "symlink failed ({symlink_error}); mklink /J failed: status={}, \
             stdout={}, stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let resolved = git_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&link))
        .expect("discover through link");
    assert!(
        resolved.path.ends_with(".auxiliary/state/fixture-app"),
        "unexpected structured path: {}",
        resolved.path.display()
    );
    let mut discovered_base = resolved.path.clone();
    for _ in 0..3 {
        assert!(discovered_base.pop(), "structured path has a base");
    }
    assert_eq!(
        strip_verbatim_prefix(
            fs::canonicalize(&discovered_base).expect("canonical base")
        ),
        strip_verbatim_prefix(
            fs::canonicalize(&repo).expect("canonical repo")
        )
    );
    assert_eq!(resolved.provenance, Provenance::DevelopmentWorktree);
}

#[cfg(windows)]
#[test]
fn windows_non_ascii_checkout_discovers() {
    let parent = temp_dir("non-ascii");
    let unicode_repo = parent.join("café-ü");
    fs::create_dir_all(&unicode_repo).expect("unicode repo");
    git(&unicode_repo, &["init", "-q", "-b", "master"]);
    let resolved = git_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&unicode_repo))
        .expect("discover unicode");
    assert_eq!(
        resolved.path,
        unicode_repo.join(".auxiliary/state/fixture-app")
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
    assert!(matches!(
        git_cargo_resolver("trusted").resolve(
            DirectoryClass::State,
            None,
            Some(&repo)
        ),
        Err(ResolutionError::IdentityMismatch { .. })
    ));
    git_cargo_resolver("foreign")
        .resolve(DirectoryClass::State, None, Some(&repo))
        .expect("real name");
}

#[test]
fn source_export_falls_through_to_platform() {
    let export = temp_dir("export");
    let resolver = Resolver::new("fixture-app").expect("resolver");
    // Every supported platform defines a state root, so the platform
    // fallthrough resolves everywhere.
    let resolved = resolver
        .resolve(DirectoryClass::State, None, Some(&export))
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
    let resolved = git_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&alias))
        .expect("discover alias");
    assert_eq!(resolved.path, repo.join(".auxiliary/state/fixture-app"));
    assert_eq!(resolved.provenance, Provenance::DevelopmentWorktree);
}

// Windows filesystems strip trailing spaces from names; Unix keeps them.
#[cfg(unix)]
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
    let resolved = git_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&repo))
        .expect("discover spaced");
    assert_eq!(resolved.path, repo.join(".auxiliary/state/fixture-app"));
}

#[cfg(target_os = "linux")]
#[test]
fn xdg_provenance_comes_from_one_snapshot() {
    // Child mode: run the assertions under the spawned environment.
    if std::env::var("APPROOTS_XDG_CHILD").as_deref() == Ok("unavailable") {
        assert!(matches!(
            approots::platform_root("fixture-app", DirectoryClass::Cache),
            Err(ResolutionError::Unavailable(_))
        ));
        return;
    }
    if std::env::var_os("APPROOTS_XDG_CHILD").is_some() {
        let resolved =
            approots::platform_root("fixture-app", DirectoryClass::Cache)
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
fn symlink_dotdot_start_keeps_semantic_resolution() {
    // alias -> repo/sub; starting at alias/.. names the repo root to
    // the OS but lexically pops to the alias parent, which is not a
    // checkout at all. Discovery must still anchor at the real repo.
    let repo = git_repo("symlink-dd");
    fs::create_dir_all(repo.join("sub")).expect("sub dir");
    let parent = temp_dir("symlink-dd-parent");
    let alias = parent.join("alias");
    std::os::unix::fs::symlink(repo.join("sub"), &alias).expect("alias");
    let start = alias.join("..");
    let resolved = git_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&start))
        .expect("discover");
    assert_eq!(resolved.path, repo.join(".auxiliary/state/fixture-app"));
    assert_eq!(resolved.provenance, Provenance::DevelopmentWorktree);
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
        let resolved = git_resolver("fixture-app")
            .resolve(DirectoryClass::State, None, Some(&repo))
            .expect("discover exotic");
        assert_eq!(resolved.path, repo.join(".auxiliary/state/fixture-app"));
    }
    // A symlinked alias containing .. still names the real checkout.
    let repo = git_repo("alias-dotdot");
    fs::create_dir_all(repo.join("a")).expect("dotdot intermediate");
    let alias_parent = temp_dir("alias-dd-parent");
    let alias = alias_parent.join("alias");
    std::os::unix::fs::symlink(&repo, &alias).expect("alias link");
    let start = alias.join("a/../");
    let resolved = git_resolver("fixture-app")
        .resolve(DirectoryClass::State, None, Some(&start))
        .expect("alias dotdot");
    assert_eq!(resolved.path, repo.join(".auxiliary/state/fixture-app"));
}
