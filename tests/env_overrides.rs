//! Opt-in per-class environment overrides, exercised in isolated
//! child processes that own their fixture environments.

mod common;

use std::{ffi::OsString, path::PathBuf};

use approots::{
    DirectoryClass, DiscovererGenus, Provenance, ResolutionError, Resolver,
};
use common::*;

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
fn exact_directory_beats_layout_base() {
    const DIR_VAR: &str = "APPROOTS_TEST_ENV_EXACT";
    const BASE_VAR: &str = "APPROOTS_TEST_ENV_BASEVAR";
    const CASE: &str = "both-shapes";
    if is_child(CASE) {
        // Both shapes declared and valid: the exact directory wins
        // while the base still validated cleanly behind it.
        let exact = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let root = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, DIR_VAR)
            .env_base(DirectoryClass::State, BASE_VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect("exact beats base");
        assert_eq!(root.path, exact);
        assert_eq!(root.provenance, Provenance::AppvarDirectory);
        return;
    }
    let exact = temp_dir("env-exact");
    let base = temp_dir("env-shape-base");
    assert_child_ok(
        spawn_self(
            "exact_directory_beats_layout_base",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(exact.clone())),
                (DIR_VAR.to_string(), string(exact)),
                (BASE_VAR.to_string(), string(base)),
            ],
        ),
        "both-shapes",
    );
}

#[test]
fn base_fills_when_exact_is_unset() {
    const DIR_VAR: &str = "APPROOTS_TEST_ENV_EXACT_ABSENT";
    const BASE_VAR: &str = "APPROOTS_TEST_ENV_BASEVAR_PRESENT";
    const CASE: &str = "base-fills";
    if is_child(CASE) {
        // Exact unset, base valid: the base composes the layout.
        let base = PathBuf::from(var("APPROOTS_ENV_BASE"));
        let root = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, DIR_VAR)
            .env_base(DirectoryClass::State, BASE_VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect("base fills");
        assert_eq!(
            root.path,
            base.join(".auxiliary").join("state").join("fixture-app")
        );
        assert_eq!(root.provenance, Provenance::AppvarBase);
        return;
    }
    let base = temp_dir("env-baseonly");
    assert_child_ok(
        spawn_self(
            "base_fills_when_exact_is_unset",
            CASE,
            &[
                ("APPROOTS_ENV_BASE".to_string(), string(base.clone())),
                (DIR_VAR.to_string(), None),
                (BASE_VAR.to_string(), string(base)),
            ],
        ),
        "base-fills",
    );
}

#[test]
fn invalid_unused_shape_still_refuses() {
    const DIR_VAR: &str = "APPROOTS_TEST_ENV_EXACT_OK";
    const BASE_VAR: &str = "APPROOTS_TEST_ENV_BASEVAR_BAD";
    const CASE: &str = "bad-shape";
    if is_child(CASE) {
        // The exact value is valid and would win, but the malformed
        // base must still refuse rather than stay silent.
        let error = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, DIR_VAR)
            .env_base(DirectoryClass::State, BASE_VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect_err("malformed base refused");
        assert!(matches!(error, ResolutionError::RelativeStartDirectory(_)));
        return;
    }
    assert_child_ok(
        spawn_self(
            "invalid_unused_shape_still_refuses",
            CASE,
            &[
                (DIR_VAR.to_string(), string(temp_dir("env-shape-ok"))),
                (BASE_VAR.to_string(), Some(OsString::from("rel/garbage"))),
            ],
        ),
        "bad-shape",
    );
}

#[test]
fn shape_order_does_not_matter() {
    const DIR_VAR: &str = "APPROOTS_TEST_ENV_ORDER_DIR";
    const BASE_VAR: &str = "APPROOTS_TEST_ENV_ORDER_BASE";
    const CASE: &str = "shape-order";
    if is_child(CASE) {
        // Base declared first, exact second: the exact directory
        // still wins by shape, not by declaration order.
        let expected = PathBuf::from(var(DIR_VAR));
        let root = Resolver::new("fixture-app")
            .expect("resolver")
            .env_base(DirectoryClass::State, BASE_VAR)
            .env_directory(DirectoryClass::State, DIR_VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect("exact beats base regardless of order");
        assert_eq!(root.path, expected);
        assert_eq!(root.provenance, Provenance::AppvarDirectory);
        return;
    }
    let exact = temp_dir("env-order-exact");
    let base = temp_dir("env-order-base");
    assert_child_ok(
        spawn_self(
            "shape_order_does_not_matter",
            CASE,
            &[
                (DIR_VAR.to_string(), string(exact)),
                (BASE_VAR.to_string(), string(base)),
            ],
        ),
        "shape-order",
    );
}

#[test]
fn invalid_exact_with_valid_base_errors() {
    const DIR_VAR: &str = "APPROOTS_TEST_ENV_BADEXACT";
    const BASE_VAR: &str = "APPROOTS_TEST_ENV_GOODBASE";
    const CASE: &str = "bad-exact";
    if is_child(CASE) {
        // The exact value is malformed while the base is valid: the
        // winner does not mask the loser's refusal.
        let error = Resolver::new("fixture-app")
            .expect("resolver")
            .env_directory(DirectoryClass::State, DIR_VAR)
            .env_base(DirectoryClass::State, BASE_VAR)
            .resolve(DirectoryClass::State, None, None)
            .expect_err("malformed exact refused");
        assert!(matches!(error, ResolutionError::RelativeStartDirectory(_)));
        return;
    }
    assert_child_ok(
        spawn_self(
            "invalid_exact_with_valid_base_errors",
            CASE,
            &[
                (DIR_VAR.to_string(), Some(OsString::from("rel/garbage"))),
                (BASE_VAR.to_string(), string(temp_dir("env-goodbase"))),
            ],
        ),
        "bad-exact",
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
        .env_directory(DirectoryClass::State, "APPROOTS_TEST_ENV_DUP_TWO")
        .resolve(DirectoryClass::State, None, None)
        .expect_err("duplicate refused");
    assert!(matches!(error, ResolutionError::DuplicateEnvDeclaration(_)));
}

#[test]
fn undeclared_class_untouched() {
    const VAR: &str = "APPROOTS_TEST_ENV_OTHER";
    const CASE: &str = "untouched";
    if is_child(CASE) {
        let declared = state_resolver(VAR);
        let plain = Resolver::new("fixture-app").expect("resolver");
        // Same class, same tier, with and without the declaration:
        // byte-identical paths and provenance. No location
        // assertion: the platform root's home is provider-defined
        // (under the fixture HOME on Unix, under AppData on
        // Windows), so only identity and provenance are asserted.
        let from_declared = declared
            .resolve(DirectoryClass::Cache, None, None)
            .expect("declared resolve");
        let from_plain = plain
            .resolve(DirectoryClass::Cache, None, None)
            .expect("plain resolve");
        assert_eq!(from_declared, from_plain);
        assert_eq!(from_declared.provenance, Provenance::DefaultUserDirectory);
        return;
    }
    let home = fixture_home();
    let mut vars = vec![];
    hermetic(&mut vars, &home);
    vars.push((VAR.to_string(), string(temp_dir("env-other"))));
    assert_child_ok(
        spawn_self("undeclared_class_untouched", CASE, &vars),
        "untouched",
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
