# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
