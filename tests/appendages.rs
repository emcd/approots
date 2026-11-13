//! Validated subpath derivation on resolved roots.

mod common;

use std::path::PathBuf;

use approots::{Provenance, ResolutionError, ResolvedRoot};
use common::*;

fn root_with(path: PathBuf, provenance: Provenance) -> ResolvedRoot {
    ResolvedRoot { path, provenance }
}

#[test]
fn join_appends_consumer_subpaths_with_provenance() {
    // Cistella shapes: single segments and multi-level subpaths, with
    // provenance inherited unchanged across all six variants.
    let base = temp_dir("append-base");
    for provenance in [
        Provenance::CallerProvided,
        Provenance::SuppliedDirectory,
        Provenance::DevelopmentCommon,
        Provenance::DevelopmentWorktree,
        Provenance::PlatformEnvironment,
        Provenance::PlatformDefault,
    ] {
        let root = root_with(base.clone(), provenance);
        let derived = root.join(&["logs", "op-9"]).expect("valid segments");
        assert_eq!(derived.path, base.join("logs").join("op-9"));
        assert_eq!(derived.provenance, provenance);
        // Byte-level check: Path equality can hide normalization, so
        // assert the receiver bytes survive verbatim as the prefix.
        let derived_bytes = derived.path.as_os_str().as_encoded_bytes();
        let base_bytes = base.as_os_str().as_encoded_bytes();
        assert!(
            derived_bytes.starts_with(base_bytes),
            "receiver must survive byte-identically"
        );
    }
    let root = root_with(base.clone(), Provenance::SuppliedDirectory);
    assert_eq!(
        root.join(&["profiles"]).expect("single segment").path,
        base.join("profiles")
    );
    assert_eq!(
        root.join(&["in-container"]).expect("dashed segment").path,
        base.join("in-container")
    );
}

#[test]
fn join_refuses_invalid_segments_by_name() {
    let base = temp_dir("append-invalid");
    let root = root_with(base, Provenance::CallerProvided);
    // Each case names its exact offender, including the first of
    // several bad tokens (validate-all-first, in order).
    for (segments, offender) in [
        (vec!["a/b"], "a/b"),
        (vec![".."], ".."),
        (vec!["."], "."),
        (vec![""], ""),
        (vec!["C:foo"], "C:foo"),
        (vec!["a:b"], "a:b"),
        (vec!["a\0b"], "a\0b"),
        (vec!["/abs"], "/abs"),
        (vec!["\\"], "\\"),
        (vec!["ok", "bad/name", "also\\bad"], "bad/name"),
    ] {
        assert!(
            matches!(
                root.join(&segments),
                Err(ResolutionError::InvalidName(ref name)) if name == offender
            ),
            "segments {segments:?} must refuse naming {offender:?}"
        );
    }
}

#[test]
fn join_empty_list_is_identity_and_receiver_is_literal() {
    let base = temp_dir("append-literal");
    let root = root_with(base.clone(), Provenance::DevelopmentWorktree);
    assert_eq!(
        root.join(&[]).expect("empty list"),
        root,
        "empty segment list returns the root unchanged"
    );
    // A symlink-sensitive base keeps its semantic form: the `..`
    // survives byte-identically instead of being popped or
    // canonicalized away.
    let semantic = base.join("child").join("..");
    let root = root_with(semantic.clone(), Provenance::CallerProvided);
    let derived = root.join(&["logs"]).expect("semantic receiver");
    let expected = {
        let mut bytes = semantic.as_os_str().to_os_string();
        bytes.push(if cfg!(windows) { "\\" } else { "/" });
        bytes.push("logs");
        PathBuf::from(bytes)
    };
    assert_eq!(derived.path.as_os_str(), expected.as_os_str());
    assert!(
        derived
            .path
            .as_os_str()
            .as_encoded_bytes()
            .windows(2)
            .any(|pair| pair == b"..")
    );
    assert_eq!(derived.provenance, Provenance::CallerProvided);
}

/// Windows-only: verbatim receivers recognise backslash solely as
/// the separator, so a trailing forward slash still needs a
/// backslash appended and `..` survives. Executes on Windows;
/// string-only, no filesystem access.
#[cfg(windows)]
#[test]
fn join_keeps_verbatim_receiver_literal() {
    let trailing =
        root_with(PathBuf::from(r"\\?\C:\base/"), Provenance::CallerProvided);
    let derived = trailing.join(&["logs"]).expect("verbatim trailing slash");
    assert_eq!(
        derived.path.as_os_str(),
        PathBuf::from(r"\\?\C:\base/\logs").as_os_str(),
        "verbatim trailing '/' needs a backslash, never fusion"
    );
    assert_eq!(derived.provenance, Provenance::CallerProvided);

    let dotted = root_with(
        PathBuf::from(r"\\?\C:\base\child\.."),
        Provenance::DevelopmentWorktree,
    );
    let derived = dotted.join(&["logs"]).expect("verbatim dotdot");
    assert_eq!(
        derived.path.as_os_str(),
        PathBuf::from(r"\\?\C:\base\child\..\logs").as_os_str(),
        "verbatim '..' must survive byte-identically"
    );
    assert_eq!(derived.provenance, Provenance::DevelopmentWorktree);
}

/// Non-Windows: a Windows-looking string keeps native separator
/// semantics. `//?/base` is an ordinary POSIX path here, so joining
/// appends `/logs` rather than emulating verbatim grammar.
#[cfg(not(windows))]
#[test]
fn join_keeps_native_separators_for_verbatim_looking_receiver() {
    let receiver =
        root_with(PathBuf::from("//?/base"), Provenance::CallerProvided);
    let derived = receiver.join(&["logs"]).expect("native join");
    assert_eq!(
        derived.path.as_os_str(),
        PathBuf::from("//?/base/logs").as_os_str(),
        "Unix must not emulate Windows verbatim grammar"
    );
    assert_eq!(derived.provenance, Provenance::CallerProvided);
}
