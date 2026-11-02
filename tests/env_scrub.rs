//! Contaminated Git environments must not divert discovery.

use std::{fs, path::PathBuf, process::Command};

use approots::discover_checkout;

#[test]
fn hostile_git_directory_does_not_divert_discovery() {
    let unique = std::process::id();
    let repo = std::env::temp_dir().join(format!("approots-scrub-{unique}"));
    fs::create_dir_all(&repo).expect("repo dir");
    let repo = repo.canonicalize().expect("canonical repo");
    for args in [
        vec!["init", "-q", "-b", "master"],
        vec!["config", "user.email", "test@example.invalid"],
        vec!["config", "user.name", "Test"],
    ] {
        let output = Command::new("git")
            .args(&args)
            .current_dir(&repo)
            .output()
            .expect("git setup");
        assert!(output.status.success(), "git {args:?} failed");
    }
    fs::write(repo.join("f.txt"), "x").expect("seed file");
    for args in [vec!["add", "."], vec!["commit", "-qm", "init"]] {
        let output = Command::new("git")
            .args(&args)
            .current_dir(&repo)
            .output()
            .expect("git setup");
        assert!(output.status.success(), "git {args:?} failed");
    }

    // SAFETY: this binary contains a single test, so no other test in
    // the process observes the mutated environment.
    unsafe {
        std::env::set_var("GIT_DIR", "/nonexistent/hostile");
        std::env::set_var("GIT_WORK_TREE", "/nonexistent");
    }
    let checkout = discover_checkout(&repo);
    // SAFETY: same single-test process as above.
    unsafe {
        std::env::remove_var("GIT_DIR");
        std::env::remove_var("GIT_WORK_TREE");
    }
    let checkout = checkout.expect("discovery survives contamination");
    assert_eq!(checkout.worktree_root, PathBuf::from(&repo));
    assert_eq!(checkout.git_common_dir, repo.join(".git"));
}
