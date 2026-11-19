//! Opt-in per-class environment overrides, exercised in isolated
//! child processes that own their fixture environments.

mod common;

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use approots::{
    DirectoryClass, DiscovererGenus, Provenance, ResolutionError, Resolver,
};
use common::*;

const CHILD_FLAG: &str = "APPROOTS_ENV_CHILD";

fn is_child(case: &str) -> bool {
    std::env::var(CHILD_FLAG).as_deref() == Ok(case)
}

/// Spawns this test binary as a child with a fixture-owned
/// environment: `Some` sets a variable, `None` removes it (proving
/// nothing leaks in from the ambient parent environment).
fn spawn_self(
    test: &str,
    case: &str,
    vars: &[(String, Option<OsString>)],
) -> std::process::Output {
    let exe = std::env::current_exe().expect("test exe");
    let mut command = Command::new(&exe);
    command.args([test, "--exact", "--nocapture"]);
    command.env(CHILD_FLAG, case);
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

fn assert_child_ok(output: std::process::Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture_home() -> PathBuf {
    temp_dir("env-home")
}

fn hermetic(vars: &mut Vec<(String, Option<OsString>)>, home: &Path) {
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

fn state_resolver(variable: &str) -> Resolver {
    Resolver::new("fixture-app")
        .expect("resolver")
        .env_directory(DirectoryClass::State, variable)
}

#[test]
fn env_directory_supplies_root() {
    const VAR: &str = "APPROOTS_TEST_ENV_STATE";
    const CASE: &str = "supply";
    if is_child(CASE) {
        let base = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let root = state_resolver(VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect("env root");
        assert_eq!(root.path, base);
        assert_eq!(root.provenance, Provenance::AppvarDirectory);
        return;
    }
    let base = temp_dir("env-supply");
    assert_child_ok(
        spawn_self(
            "env_directory_supplies_root",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(base.clone())),
                (VAR.to_string(), string(base)),
            ],
        ),
        "supply",
    );
}

#[test]
fn env_base_composes_layout() {
    const VAR: &str = "APPROOTS_TEST_ENV_PREFIX";
    const CASE: &str = "base";
    if is_child(CASE) {
        let base = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let root = Resolver::new("fixture-app")
            .expect("resolver")
            .env_base(DirectoryClass::State, VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect("env base root");
        assert_eq!(
            root.path,
            base.join(".auxiliary").join("state").join("fixture-app")
        );
        assert_eq!(root.provenance, Provenance::AppvarBase);
        return;
    }
    let base = temp_dir("env-base");
    assert_child_ok(
        spawn_self(
            "env_base_composes_layout",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(base.clone())),
                (VAR.to_string(), string(base)),
            ],
        ),
        "base",
    );
}

#[test]
fn explicit_masks_declaration_problems() {
    // Explicit wins without capture, so even declaration problems
    // for the masked class never surface.
    let explicit = temp_dir("env-maskdecl");
    let masked = |resolve: Resolver| {
        resolve
            .resolve(DirectoryClass::State, Some(&explicit), None)
            .expect("explicit masks declaration")
    };
    let root = masked(
        Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, ""),
    );
    assert_eq!(root.provenance, Provenance::ExplicitDirectory);
    let root = masked(
        Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, "APPROOTS_TEST_ENV_X")
            .env_base(DirectoryClass::State, "APPROOTS_TEST_ENV_Y"),
    );
    assert_eq!(root.provenance, Provenance::ExplicitDirectory);
}

#[test]
fn explicit_beats_and_masks_env() {
    const VAR: &str = "APPROOTS_TEST_ENV_MASKED";
    const CASE: &str = "mask";
    if is_child(CASE) {
        // Same-kind malformed value is never read: no start, no
        // error, explicit wins.
        let explicit = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let root = state_resolver(VAR)
            .resolve(DirectoryClass::State, Some(&explicit), None)
            .expect("explicit root");
        assert_eq!(root.path, explicit);
        assert_eq!(root.provenance, Provenance::ExplicitDirectory);
        return;
    }
    let explicit = temp_dir("env-explicit");
    assert_child_ok(
        spawn_self(
            "explicit_beats_and_masks_env",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(explicit)),
                (VAR.to_string(), Some(OsString::from("rel/garbage"))),
            ],
        ),
        "mask",
    );
}

#[test]
fn unset_falls_through() {
    const VAR: &str = "APPROOTS_TEST_ENV_ABSENT";
    const CASE: &str = "absent";
    if is_child(CASE) {
        let root = state_resolver(VAR)
            .resolve(DirectoryClass::Cache, None, None)
            .expect("platform fallback");
        // Undeclared class: platform tier, never the environment.
        assert_eq!(root.provenance, Provenance::DefaultUserDirectory);
        return;
    }
    let home = fixture_home();
    // Unset: the variable is removed, proving nothing leaks in.
    let mut vars = vec![];
    hermetic(&mut vars, &home);
    vars.push((VAR.to_string(), None));
    assert_child_ok(spawn_self("unset_falls_through", CASE, &vars), "unset");
}

#[test]
fn zero_length_is_malformed() {
    const VAR: &str = "APPROOTS_TEST_ENV_EMPTY";
    const CASE: &str = "zero-length";
    if is_child(CASE) {
        // Omissibility is not nullability: an explicitly set empty
        // value fails loudly instead of silently falling through.
        let error = state_resolver(VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect_err("empty env refused");
        assert!(matches!(error, ResolutionError::EmptyPath));
        return;
    }
    assert_child_ok(
        spawn_self(
            "zero_length_is_malformed",
            CASE,
            &[(VAR.to_string(), Some(OsString::from("")))],
        ),
        "zero-length",
    );
}

#[test]
fn whitespace_value_validates_as_relative() {
    const VAR: &str = "APPROOTS_TEST_ENV_SPACE";
    const CASE: &str = "space";
    if is_child(CASE) {
        // Whitespace-only is non-empty: it validates as a relative
        // value, joining against start or erroring without one.
        let start = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let root = state_resolver(VAR)
            .resolve(DirectoryClass::State, None, Some(&start))
            .expect("whitespace joins start");
        assert_eq!(root.path, start.join("  "));
        assert_eq!(root.provenance, Provenance::AppvarDirectory);
        let error = state_resolver(VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect_err("whitespace needs start");
        assert!(matches!(error, ResolutionError::RelativeStartDirectory(_)));
        return;
    }
    let start = temp_dir("env-space");
    assert_child_ok(
        spawn_self(
            "whitespace_value_validates_as_relative",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(start)),
                (VAR.to_string(), Some(OsString::from("  "))),
            ],
        ),
        "space",
    );
}

#[test]
fn invalid_declarations_refused() {
    // Declaration checks never touch the environment, so these run
    // in-process with names nothing else could claim.
    let error = Resolver::new("fixture-app")
        .expect("resolver")
        .env_directory(DirectoryClass::State, "")
        .resolve(DirectoryClass::State, None, None)
        .expect_err("empty name refused");
    assert!(matches!(
        error,
        ResolutionError::InvalidEnvDeclaration { .. }
    ));
    let error = Resolver::new("fixture-app")
        .expect("resolver")
        .env_directory(DirectoryClass::State, "HAS=EQUALS")
        .resolve(DirectoryClass::State, None, None)
        .expect_err("equals name refused");
    assert!(matches!(
        error,
        ResolutionError::InvalidEnvDeclaration { .. }
    ));
    let error = Resolver::new("fixture-app")
        .expect("resolver")
        .env_directory(DirectoryClass::State, "APPROOTS_TEST_ENV_DUP")
        .env_base(DirectoryClass::State, "APPROOTS_TEST_ENV_DUP_TWO")
        .resolve(DirectoryClass::State, None, None)
        .expect_err("duplicate refused");
    assert!(matches!(error, ResolutionError::DuplicateEnvDeclaration(_)));
}

#[test]
fn undeclared_class_untouched() {
    const VAR: &str = "APPROOTS_TEST_ENV_OTHER";
    const CASE: &str = "untouched";
    if is_child(CASE) {
        let base = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let declared = state_resolver(VAR);
        let plain = Resolver::new("fixture-app").expect("resolver");
        // Same class, same tier, with and without the declaration:
        // byte-identical paths and provenance.
        let from_declared = declared
            .resolve(DirectoryClass::Cache, None, None)
            .expect("declared resolve");
        let from_plain = plain
            .resolve(DirectoryClass::Cache, None, None)
            .expect("plain resolve");
        assert_eq!(from_declared, from_plain);
        assert_eq!(from_declared.provenance, Provenance::DefaultUserDirectory);
        assert!(from_declared.path.starts_with(&base));
        return;
    }
    let home = fixture_home();
    let mut vars =
        vec![("APPROOTS_ENV_BASE".to_string(), string(home.clone()))];
    hermetic(&mut vars, &home);
    vars.push((VAR.to_string(), string(temp_dir("env-other"))));
    assert_child_ok(
        spawn_self("undeclared_class_untouched", CASE, &vars),
        "untouched",
    );
}

#[test]
fn family_gap_fills_from_env() {
    const VAR: &str = "APPROOTS_TEST_ENV_FAMILY";
    const CASE: &str = "family-gap";
    if is_child(CASE) {
        use approots::PartialRoots;
        let state = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let partial = PartialRoots {
            state: None,
            ..Default::default()
        };
        let bundle = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, VAR)
            .resolve_partially_explicit(&partial, None)
            .expect("family bundle");
        assert_eq!(bundle.state.path, state);
        assert_eq!(bundle.state.provenance, Provenance::AppvarDirectory);
        // Other gaps fall back to platform with no start.
        assert_eq!(bundle.cache.provenance, Provenance::DefaultUserDirectory);
        return;
    }
    let home = fixture_home();
    let state = temp_dir("env-family-state");
    let mut vars =
        vec![("APPROOTS_ENV_BASE".to_string(), string(state.clone()))];
    hermetic(&mut vars, &home);
    vars.push((VAR.to_string(), string(state)));
    assert_child_ok(
        spawn_self("family_gap_fills_from_env", CASE, &vars),
        "family-gap",
    );
}

#[test]
fn family_all_filled_masks_env() {
    const VAR: &str = "APPROOTS_TEST_ENV_ALLFILLED";
    const CASE: &str = "all-filled";
    if is_child(CASE) {
        use approots::PartialRoots;
        let complete = PathBuf::from(var("APPROOTS_ENV_BASE"));
        // Even an invalid declared value is never consulted: the
        // all-filled bundle is a pure validator.
        let partial = PartialRoots {
            configuration: Some(complete.clone()),
            state: Some(complete.clone()),
            cache: Some(complete.clone()),
            data: Some(complete),
        };
        let bundle = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, VAR)
            .resolve_partially_explicit(&partial, None)
            .expect("all-filled bundle");
        assert_eq!(bundle.state.provenance, Provenance::ExplicitDirectory);
        return;
    }
    let complete = temp_dir("env-allfilled");
    assert_child_ok(
        spawn_self(
            "family_all_filled_masks_env",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(complete)),
                (VAR.to_string(), Some(OsString::from("rel/garbage"))),
            ],
        ),
        "all-filled",
    );
}

#[test]
fn family_duplicate_declaration_fails_before_discovery() {
    const VAR: &str = "APPROOTS_TEST_ENV_BADGAP";
    const CASE: &str = "bad-gap";
    if is_child(CASE) {
        use approots::PartialRoots;
        let start = PathBuf::from(var("APPROOTS_ENV_BASE"));
        // Discovery would succeed here (a real checkout), so a
        // typed refusal proves declaration checks run before any
        // gap discovery work.
        let error = git_resolver("fixture-app")
            .env_directory(DirectoryClass::Cache, VAR)
            .env_base(DirectoryClass::Cache, "APPROOTS_TEST_ENV_BADGAP_TWO")
            .resolve_partially_explicit(&PartialRoots::default(), Some(&start))
            .expect_err("duplicate gap declaration refused");
        assert!(matches!(error, ResolutionError::DuplicateEnvDeclaration(_)));
        return;
    }
    let repo = git_repo("env-badgap");
    assert_child_ok(
        spawn_self(
            "family_duplicate_declaration_fails_before_discovery",
            CASE,
            &[("APPROOTS_ENV_BASE".to_string(), string(repo))],
        ),
        "bad-gap",
    );
}

#[test]
fn family_malformed_relative_without_start_errors() {
    const VAR: &str = "APPROOTS_TEST_ENV_NOSTARTREL";
    const CASE: &str = "no-start-rel";
    if is_child(CASE) {
        use approots::PartialRoots;
        // No start and a relative value: the typed refusal returns
        // instead of the platform fallback.
        let error = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::Cache, VAR)
            .resolve_partially_explicit(&PartialRoots::default(), None)
            .expect_err("relative without start refused");
        assert!(matches!(error, ResolutionError::RelativeStartDirectory(_)));
        return;
    }
    assert_child_ok(
        spawn_self(
            "family_malformed_relative_without_start_errors",
            CASE,
            &[(VAR.to_string(), Some(OsString::from("rel/garbage")))],
        ),
        "no-start-rel",
    );
}

#[test]
fn single_argument_validation_runs_when_env_wins() {
    const VAR: &str = "APPROOTS_TEST_ENV_ARGS";
    const CASE: &str = "args";
    if is_child(CASE) {
        // An orphan argument bag is a contract error even though the
        // environment tier would have supplied the root.
        let error = state_resolver(VAR)
            .discoverer_argument(
                DiscovererGenus::Vcs,
                "scope".to_string(),
                "common".to_string(),
            )
            .resolve(DirectoryClass::State, None, None)
            .expect_err("orphan bag refused");
        assert!(matches!(
            error,
            ResolutionError::InvalidDiscovererArgument { .. }
        ));
        return;
    }
    assert_child_ok(
        spawn_self(
            "single_argument_validation_runs_when_env_wins",
            CASE,
            &[(VAR.to_string(), string(temp_dir("env-args")))],
        ),
        "args",
    );
}

#[test]
fn env_without_start_supplies_absolute() {
    const VAR: &str = "APPROOTS_TEST_ENV_NOSTART";
    const CASE: &str = "no-start";
    if is_child(CASE) {
        // No start, no discovery possible: absolute environment
        // value still supplies the root instead of the fallback.
        let base = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let root = state_resolver(VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect("env root without start");
        assert_eq!(root.path, base);
        assert_eq!(root.provenance, Provenance::AppvarDirectory);
        return;
    }
    let base = temp_dir("env-nostart");
    assert_child_ok(
        spawn_self(
            "env_without_start_supplies_absolute",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(base.clone())),
                (VAR.to_string(), string(base)),
            ],
        ),
        "no-start",
    );
}

#[test]
fn poisoned_ambient_env_does_not_divert_env_tier() {
    const VAR: &str = "APPROOTS_TEST_ENV_CLEAN";
    const CASE: &str = "poison";
    if is_child(CASE) {
        let base = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let root = state_resolver(VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect("env root through poison");
        assert_eq!(root.path, base);
        assert_eq!(root.provenance, Provenance::AppvarDirectory);
        return;
    }
    let base = temp_dir("env-poison");
    assert_child_ok(
        spawn_self(
            "poisoned_ambient_env_does_not_divert_env_tier",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(base.clone())),
                (VAR.to_string(), string(base)),
                (
                    "APPROOTS_TEST_POISON_ONE".to_string(),
                    Some(OsString::from("junk")),
                ),
                (
                    "GIT_DIR".to_string(),
                    Some(OsString::from("/nonexistent-poison")),
                ),
            ],
        ),
        "poison",
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_value_preserved() {
    const VAR: &str = "APPROOTS_TEST_ENV_NONUTF8";
    const CASE: &str = "nonutf8";
    if is_child(CASE) {
        use std::os::unix::ffi::OsStringExt;
        let prefix = std::env::var_os("APPROOTS_ENV_BASE").expect("base");
        let mut bytes = prefix.as_encoded_bytes().to_vec();
        bytes.push(0xFF);
        let raw = OsString::from_vec(bytes);
        // Name the case after the value: the variable itself is set
        // by the parent to the same non-UTF8 bytes.
        let root = state_resolver(VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect("non-utf8 root");
        assert_eq!(root.path.as_os_str(), &raw);
        assert_eq!(root.provenance, Provenance::AppvarDirectory);
        return;
    }
    use std::os::unix::ffi::OsStringExt;
    let prefix = temp_dir("env-nonutf8");
    let mut bytes = prefix.as_os_str().as_encoded_bytes().to_vec();
    bytes.push(0xFF);
    let raw = OsString::from_vec(bytes);
    assert_child_ok(
        spawn_self(
            "non_utf8_value_preserved",
            CASE,
            &[
                (
                    "APPROOTS_ENV_BASE".to_string(),
                    Some(prefix.as_os_str().to_os_string()),
                ),
                (VAR.to_string(), Some(raw)),
            ],
        ),
        "nonutf8",
    );
}

#[cfg(windows)]
#[test]
fn windows_drive_relative_env_refused() {
    const VAR: &str = "APPROOTS_TEST_ENV_WINREL";
    const CASE: &str = "winrel";
    if is_child(CASE) {
        let error = state_resolver(VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect_err("drive-relative refused");
        assert!(matches!(error, ResolutionError::InvalidName(_)));
        return;
    }
    assert_child_ok(
        spawn_self(
            "windows_drive_relative_env_refused",
            CASE,
            &[(VAR.to_string(), Some(OsString::from("C:foo")))],
        ),
        "winrel",
    );
}

/// A discoverer that records every invocation, so tests can prove
/// discovery did or did not run. Returns the start directory itself
/// as the discovered base.
#[derive(Debug, Clone, Default)]
struct RecordingDiscoverer {
    calls: std::sync::Arc<std::sync::Mutex<Vec<PathBuf>>>,
}

impl approots::Discoverer for RecordingDiscoverer {
    fn genus(&self) -> DiscovererGenus {
        DiscovererGenus::Vcs
    }
}

impl approots::VcsDiscoverer for RecordingDiscoverer {
    fn validate_args(
        &self,
        _args: &std::collections::BTreeMap<String, String>,
    ) -> Result<(), ResolutionError> {
        Ok(())
    }

    fn discover(
        &self,
        start: &std::path::Path,
        _args: &std::collections::BTreeMap<String, String>,
    ) -> Result<Option<approots::DiscoveredBase>, ResolutionError> {
        self.calls.lock().expect("calls").push(start.to_path_buf());
        Ok(Some(approots::DiscoveredBase {
            path: start.to_path_buf(),
            provenance: Provenance::Discovery,
        }))
    }

    fn clone_box(&self) -> Box<dyn approots::VcsDiscoverer> {
        Box::new(self.clone())
    }
}

#[test]
fn environment_barrier_runs_before_any_discovery() {
    const VAR: &str = "APPROOTS_TEST_ENV_BARRIER";
    const CASE: &str = "barrier";
    if is_child(CASE) {
        use approots::PartialRoots;
        let recorder = RecordingDiscoverer::default();
        let calls = recorder.calls.clone();
        // State would discover cleanly, but the Cache gap's empty
        // value must fail first — with zero discoverer invocations.
        let error = Resolver::new("fixture-app")
            .expect("resolver")
            .register_vcs(recorder)
            .expect("register vcs")
            .env_directory(DirectoryClass::Cache, VAR)
            .resolve_partially_explicit(
                &PartialRoots::default(),
                Some(&PathBuf::from(var("APPROOTS_ENV_BASE"))),
            )
            .expect_err("empty gap env refused");
        assert!(matches!(error, ResolutionError::EmptyPath));
        assert!(
            calls.lock().expect("calls").is_empty(),
            "barrier runs before any discovery"
        );
        return;
    }
    let start = temp_dir("env-barrier");
    assert_child_ok(
        spawn_self(
            "environment_barrier_runs_before_any_discovery",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(start)),
                (VAR.to_string(), Some(OsString::from(""))),
            ],
        ),
        "barrier",
    );
}

#[test]
fn family_argument_validation_runs_when_env_wins() {
    const VAR: &str = "APPROOTS_TEST_ENV_FAMARGS";
    const CASE: &str = "famargs";
    if is_child(CASE) {
        use approots::PartialRoots;
        // A gap's valid environment value does not suppress argument
        // validation: the orphan bag still refuses through the actual
        // family API.
        let error = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, VAR)
            .discoverer_argument(
                DiscovererGenus::Vcs,
                "scope".to_string(),
                "common".to_string(),
            )
            .resolve_partially_explicit(&PartialRoots::default(), None)
            .expect_err("orphan bag refused");
        assert!(matches!(
            error,
            ResolutionError::InvalidDiscovererArgument { .. }
        ));
        return;
    }
    assert_child_ok(
        spawn_self(
            "family_argument_validation_runs_when_env_wins",
            CASE,
            &[(VAR.to_string(), string(temp_dir("env-famargs")))],
        ),
        "famargs",
    );
}

#[test]
fn some_masks_malformed_same_kind_env() {
    const VAR: &str = "APPROOTS_TEST_ENV_SOMEMASK";
    const STATE_VAR: &str = "APPROOTS_TEST_ENV_SOMEMASK_STATE";
    const CASE: &str = "some-mask";
    if is_child(CASE) {
        use approots::PartialRoots;
        // The Configuration Some value is valid while its declared
        // variable is malformed: the Some wins and the malformed
        // value is never read.
        let complete = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let partial = PartialRoots {
            configuration: Some(complete.clone()),
            ..Default::default()
        };
        let bundle = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::Configuration, VAR)
            .env_directory(DirectoryClass::State, STATE_VAR)
            .resolve_partially_explicit(&partial, None)
            .expect("some masks same-kind env");
        assert_eq!(
            bundle.configuration.provenance,
            Provenance::ExplicitDirectory
        );
        assert_eq!(bundle.configuration.path, complete);
        assert_eq!(bundle.state.provenance, Provenance::AppvarDirectory);
        return;
    }
    let complete = temp_dir("env-somemask");
    let state = temp_dir("env-somemask-state");
    assert_child_ok(
        spawn_self(
            "some_masks_malformed_same_kind_env",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(complete.clone())),
                (
                    VAR.to_string(),
                    Some(OsString::from("rel/garbage-no-start")),
                ),
                (STATE_VAR.to_string(), string(state)),
            ],
        ),
        "some-mask",
    );
}

#[test]
fn shared_variable_supplies_two_classes() {
    const VAR: &str = "APPROOTS_TEST_ENV_SHARED";
    const CASE: &str = "shared";
    if is_child(CASE) {
        use approots::PartialRoots;
        // One variable name backing two classes: each class reads it
        // independently.
        let shared = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let bundle = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, VAR)
            .env_directory(DirectoryClass::Cache, VAR)
            .resolve_partially_explicit(&PartialRoots::default(), None)
            .expect("shared variable supplies both");
        assert_eq!(bundle.state.path, shared);
        assert_eq!(bundle.cache.path, shared);
        assert_eq!(bundle.state.provenance, Provenance::AppvarDirectory);
        assert_eq!(bundle.cache.provenance, Provenance::AppvarDirectory);
        return;
    }
    let shared = temp_dir("env-shared");
    assert_child_ok(
        spawn_self(
            "shared_variable_supplies_two_classes",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(shared.clone())),
                (VAR.to_string(), string(shared)),
            ],
        ),
        "shared",
    );
}

fn var(name: &str) -> String {
    std::env::var(name).expect("fixture var")
}

fn string(path: PathBuf) -> Option<OsString> {
    Some(path.into_os_string())
}
