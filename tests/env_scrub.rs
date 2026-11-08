//! Contaminated Git environments must not divert discovery.

use std::{fs, path::PathBuf, process::Command};

use approots::discover_checkout;

/// Removes the Windows verbatim prefix (`\\?\`) that canonicalization
/// adds, since Git reports its own results without the prefix.
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

#[test]
fn hostile_git_directory_does_not_divert_discovery() {
    let unique = std::process::id();
    let repo = std::env::temp_dir().join(format!("approots-scrub-{unique}"));
    fs::create_dir_all(&repo).expect("repo dir");
    let repo =
        strip_verbatim_prefix(repo.canonicalize().expect("canonical repo"));
    for args in [
        vec!["init", "-q", "-b", "master"],
        vec!["config", "user.email", "test@example.invalid"],
        vec!["config", "user.name", "Test"],
        vec!["add", "."],
        vec!["commit", "-qm", "init"],
    ] {
        // Fixture-init git calls get the same prefix blast as the
        // library: hooks and CI export GIT_* routing variables.
        let mut command = Command::new("git");
        command.args(&args).current_dir(&repo);
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("GIT_") {
                command.env_remove(&key);
            }
        }
        command.env("GIT_CONFIG_NOSYSTEM", "1");
        if args[0] == "add" {
            fs::write(repo.join("f.txt"), "x").expect("seed file");
        }
        let output = command.output().expect("git setup");
        assert!(output.status.success(), "git {args:?} failed");
    }

    // SAFETY: this binary contains a single test, so no other test in
    // the process observes the mutated environment.
    unsafe {
        std::env::set_var("GIT_DIR", "/nonexistent/hostile");
        std::env::set_var("GIT_WORK_TREE", "/nonexistent");
        std::env::set_var("GIT_INDEX_FILE", "/nonexistent/index");
        std::env::set_var("GIT_COMMON_DIR", "/nonexistent/common");
        std::env::set_var("GIT_OBJECT_DIRECTORY", "/nonexistent/objects");
        std::env::set_var(
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "/nonexistent/alt-objects",
        );
    }
    let checkout = discover_checkout(&repo);
    // SAFETY: same single-test process as above.
    unsafe {
        std::env::remove_var("GIT_DIR");
        std::env::remove_var("GIT_WORK_TREE");
        std::env::remove_var("GIT_INDEX_FILE");
        std::env::remove_var("GIT_COMMON_DIR");
        std::env::remove_var("GIT_OBJECT_DIRECTORY");
        std::env::remove_var("GIT_ALTERNATE_OBJECT_DIRECTORIES");
    }
    let checkout = checkout.expect("discovery survives contamination");
    assert_eq!(checkout.worktree_root, PathBuf::from(&repo));
    assert_eq!(checkout.git_common_dir, repo.join(".git"));

    // Missing Git is an explicit outcome, never a silent fallback.
    let saved_path = std::env::var_os("PATH");
    // SAFETY: same single-test process as above.
    unsafe {
        std::env::set_var("PATH", "/nonexistent/empty-path");
    }
    let missing = approots::discover_checkout(&repo);
    assert!(matches!(
        missing,
        Err(approots::ResolutionError::GitUnavailable)
    ));

    // Through Resolver the missing executable surfaces identically;
    // it never becomes a silent platform fallback.
    let resolver = approots::Resolver::new("fixture-app")
        .expect("resolver")
        .development_mode(true);
    // SAFETY: same single-test process as above.
    unsafe {
        std::env::set_var("PATH", "/nonexistent/empty-path");
    }
    let via_resolver = resolver.resolve(
        approots::DirectoryKind::State,
        None,
        Some(&repo),
        approots::DevelopmentScope::Worktree,
        None,
    );
    // SAFETY: same single-test process as above.
    unsafe {
        match saved_path {
            Some(path) => std::env::set_var("PATH", path),
            None => std::env::remove_var("PATH"),
        }
    }
    assert!(matches!(
        via_resolver,
        Err(approots::ResolutionError::GitUnavailable)
    ));
}
