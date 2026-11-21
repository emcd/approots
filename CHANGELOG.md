# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1] - 2026-11-21

### Fixed

- Exact directory beats layout base when one class declares both
  environment shapes, with fail-fast validation of the unused
  shape: precedence is now caller-supplied explicit root, declared
  exact directory, declared layout base, discovery result,
  platform-standard root.
- `DuplicateEnvDeclaration` now fires only for two declarations with
  the same shape for one class; declaring both shapes is allowed.

## [0.4.0] - 2026-11-20

### Changed

- **BREAKING**: the closed provenance vocabulary is renamed to a
  source-first series: `CallerProvided` → `ExplicitDirectory`,
  `SuppliedDirectory` → `ExplicitBase`,
  `DevelopmentCommon`/`DevelopmentWorktree` → `Discovery`,
  `PlatformEnvironment` → `SysvarUserDirectory`,
  `PlatformDefault` → `DefaultUserDirectory`. Resolved paths are
  unchanged; only the reported values change, and downstream
  exhaustive matches must be updated.

### Added

- Opt-in per-class environment overrides: `Resolver::env_directory`
  and `Resolver::env_base` declare one variable per class, read once
  with native preservation. Precedence is explicit → environment →
  discovery → platform, reported as `AppvarDirectory`/`AppvarBase`.
  An unset variable falls through; a zero-length value is an
  `EmptyPath` refusal rather than silent absence.

## [0.3.0] - 2026-11-14

### Added

- Validated subpath appendages: `ResolvedRoot::join` derives subpaths
  beneath a resolved root from caller-supplied segments, enforcing the
  shared token grammar with first-offender errors, appending at the
  byte level so the receiver survives literally, and inheriting the
  receiver's provenance unchanged.

## [0.2.0] - 2026-11-09

### Added

- Family partial-explicit resolution: `PartialRoots` input and
  per-kind `ResolvedRoot` output structs plus
  `Resolver::resolve_partially_explicit`, validating supplied
  complete directories per kind and filling gaps through discovery
  or platform defaults.

## [0.1.1] - 2026-11-09

### Changed

- Rewrite the README introduction in plain language and add a usage
  example.

## [0.1.0] - 2026-11-08

### Added

- Initial release: caller-driven, read-only directory resolution with
  explicit, discovered, and platform tiers plus provenance reporting.
- Git checkout discovery with shared-vs-local scope and `Cargo.toml`
  identity validation.
- `LocalStructure` composition, per-genus discoverers, closed
  configuration layer lists, and shared name sanitization.
- Platform-standard roots on Linux, macOS, and Windows with an opt-in
  distinct-directory-classes layout.
