//! Shared fixtures for the behavioral test binaries.
//!
//! Helpers are shared across binaries, so per-binary dead-code analysis
//! would flag helpers used only elsewhere.
#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use approots::{
    DevfileDiscovererCargo, DiscovererGenus, Resolver, VcsDiscovererGit,
};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn temp_dir(tag: &str) -> PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir()
        .join(format!("approots-{tag}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&path).expect("create temp dir");
    strip_verbatim_prefix(path.canonicalize().expect("canonical temp dir"))
}

/// Removes the Windows verbatim prefix that canonicalization adds,
/// since Git cannot create worktrees beneath verbatim paths and
/// reports its own results without the prefix.
pub fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path
}

pub fn git(dir: &Path, args: &[&str]) {
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

pub fn git_repo(tag: &str) -> PathBuf {
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

/// A resolver with only the Git discoverer registered.
pub fn git_resolver(app: &str) -> Resolver {
    Resolver::new(app)
        .expect("resolver")
        .register_vcs(VcsDiscovererGit)
        .expect("register vcs")
}

/// A resolver with Git and Cargo discoverers registered.
pub fn git_cargo_resolver(app: &str) -> Resolver {
    git_resolver(app)
        .register_devfile(DevfileDiscovererCargo)
        .expect("register devfile")
}

/// A resolver with only the Cargo discoverer registered.
pub fn cargo_resolver(app: &str) -> Resolver {
    Resolver::new(app)
        .expect("resolver")
        .register_devfile(DevfileDiscovererCargo)
        .expect("register devfile")
}

pub fn scope_arg(resolver: Resolver, scope: &str) -> Resolver {
    resolver.discoverer_argument(
        DiscovererGenus::Vcs,
        VcsDiscovererGit::SCOPE_KEY,
        scope,
    )
}
