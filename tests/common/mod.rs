//! Shared fixtures for the behavioral test binaries.
//!
//! Helpers are shared across binaries, so per-binary dead-code analysis
//! would flag helpers used only elsewhere.
#![allow(dead_code)]

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use approots::{
    DevfileDiscovererCargo, DirectoryClass, DiscovererGenus, Resolver,
    VcsDiscovererGit,
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

/// Child-process flag shared by the isolated environment tests.
pub const CHILD_ENV_FLAG: &str = "APPROOTS_ENV_CHILD";

/// True inside a spawned child for the given case.
pub fn is_child(case: &str) -> bool {
    std::env::var(CHILD_ENV_FLAG).as_deref() == Ok(case)
}

/// Spawns the current test binary as a child with a fixture-owned
/// environment: `Some` sets a variable, `None` removes it (proving
/// nothing leaks in from the ambient parent environment).
pub fn spawn_self(
    test: &str,
    case: &str,
    vars: &[(String, Option<OsString>)],
) -> std::process::Output {
    let exe = std::env::current_exe().expect("test exe");
    let mut command = Command::new(&exe);
    command.args([test, "--exact", "--nocapture"]);
    command.env(CHILD_ENV_FLAG, case);
    for (key, value) in vars {
        match value {
            Some(bytes) => {
                command.env(key, bytes);
            },
            None => {
                command.env_remove(key);
            },
        }
    }
    command.output().expect("spawn env child")
}

pub fn assert_child_ok(output: std::process::Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn fixture_home() -> PathBuf {
    temp_dir("env-home")
}

pub fn hermetic(vars: &mut Vec<(String, Option<OsString>)>, home: &Path) {
    // Platform fallbacks must resolve beneath a fixture home, never
    // the runner's real one; XDG overrides are removed outright.
    vars.push(("HOME".to_string(), Some(home.as_os_str().to_os_string())));
    for key in [
        "XDG_CACHE_HOME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
    ] {
        vars.push((key.to_string(), None));
    }
}

pub fn state_resolver(variable: &str) -> Resolver {
    Resolver::new("fixture-app")
        .expect("resolver")
        .env_directory(DirectoryClass::State, variable)
}

pub fn var(name: &str) -> String {
    std::env::var(name).expect("fixture var")
}

pub fn string(path: PathBuf) -> Option<OsString> {
    Some(path.into_os_string())
}
