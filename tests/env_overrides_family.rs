//! Family-scoped environment overrides, exercised in isolated
//! child processes that own their fixture environments.

mod common;

use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use approots::{
    DirectoryClass, DiscoveredBase, Discoverer, DiscovererGenus, Provenance,
    ResolutionError, Resolver, VcsDiscoverer,
};
use common::*;

/// A discoverer that records every invocation, so tests can prove
/// discovery did or did not run. Returns the start directory itself
/// as the discovered base.
#[derive(Debug, Clone, Default)]
struct RecordingDiscoverer {
    calls: Arc<Mutex<Vec<PathBuf>>>,
}

impl Discoverer for RecordingDiscoverer {
    fn genus(&self) -> DiscovererGenus {
        DiscovererGenus::Vcs
    }
}

impl VcsDiscoverer for RecordingDiscoverer {
    fn validate_args(
        &self,
        _args: &BTreeMap<String, String>,
    ) -> Result<(), ResolutionError> {
        Ok(())
    }

    fn discover(
        &self,
        start: &Path,
        _args: &BTreeMap<String, String>,
    ) -> Result<Option<DiscoveredBase>, ResolutionError> {
        self.calls.lock().expect("calls").push(start.to_path_buf());
        Ok(Some(DiscoveredBase {
            path: start.to_path_buf(),
            provenance: Provenance::Discovery,
        }))
    }

    fn clone_box(&self) -> Box<dyn VcsDiscoverer> {
        Box::new(self.clone())
    }
}

#[test]
fn family_gap_prefers_exact_over_base() {
    const DIR_VAR: &str = "APPROOTS_TEST_ENV_FAMEXACT";
    const BASE_VAR: &str = "APPROOTS_TEST_ENV_FAMBASE";
    const CASE: &str = "fam-shape";
    if is_child(CASE) {
        use approots::PartialRoots;
        // Base declared before exact: the exact directory still
        // wins by shape, not by declaration order.
        let exact = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let bundle = Resolver::new("fixture-app")
            .expect("resolver")
            .env_base(DirectoryClass::State, BASE_VAR)
            .env_directory(DirectoryClass::State, DIR_VAR)
            .resolve_partially_explicit(&PartialRoots::default(), None)
            .expect("gap prefers exact");
        assert_eq!(bundle.state.path, exact);
        assert_eq!(bundle.state.provenance, Provenance::AppvarDirectory);
        return;
    }
    let exact = temp_dir("env-famexact");
    let base = temp_dir("env-fambase");
    assert_child_ok(
        spawn_self(
            "family_gap_prefers_exact_over_base",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(exact.clone())),
                (DIR_VAR.to_string(), string(exact)),
                (BASE_VAR.to_string(), string(base)),
            ],
        ),
        "fam-shape",
    );
}

#[test]
fn family_malformed_unused_shape_errors_at_barrier() {
    const DIR_VAR: &str = "APPROOTS_TEST_ENV_FAMBADDIR";
    const BASE_VAR: &str = "APPROOTS_TEST_ENV_FAMBADBASE";
    const CASE: &str = "fam-badshape";
    if is_child(CASE) {
        use approots::PartialRoots;
        // The exact value is valid and would win, but a malformed
        // base for the same gap still refuses at the family barrier.
        let error = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, DIR_VAR)
            .env_base(DirectoryClass::State, BASE_VAR)
            .resolve_partially_explicit(&PartialRoots::default(), None)
            .expect_err("malformed unused shape refused");
        assert!(matches!(error, ResolutionError::RelativeStartDirectory(_)));
        return;
    }
    let exact = temp_dir("env-famgood");
    assert_child_ok(
        spawn_self(
            "family_malformed_unused_shape_errors_at_barrier",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(exact.clone())),
                (DIR_VAR.to_string(), string(exact)),
                (BASE_VAR.to_string(), Some(OsString::from("rel/garbage"))),
            ],
        ),
        "fam-badshape",
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
            .env_directory(
                DirectoryClass::Cache,
                "APPROOTS_TEST_ENV_BADGAP_TWO",
            )
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
        // The Configuration Some value is valid while BOTH of its
        // declared variables are malformed: the Some wins and
        // neither shape is ever read.
        let complete = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let partial = PartialRoots {
            configuration: Some(complete.clone()),
            ..Default::default()
        };
        let bundle = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::Configuration, VAR)
            .env_base(
                DirectoryClass::Configuration,
                "APPROOTS_TEST_ENV_BADBASE",
            )
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
                (
                    "APPROOTS_TEST_ENV_BADBASE".to_string(),
                    Some(OsString::from("also/relative-garbage")),
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
