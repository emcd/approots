# family-resolution

## Purpose

Whole-family resolution with per-kind complete-or-missing roots: one call validates a caller-packaged family and fills its gaps through the normal chain.

## Requirements

### Requirement: Family partial-explicit resolution

The library SHALL provide family input/output structs and a `Resolver::resolve_partially_explicit` method settling all four directory classes in one call. The input (`PartialRoots`) SHALL carry one `Option<PathBuf>` per class; each `Some` value SHALL be a complete directory validated exactly like the explicit tier — absolute and non-empty, refusing Windows ambient-dependent shapes, joining a relative value against `start` (typed error when relative with no start) — reported with `CallerProvided` provenance and never composed further. Each `None` value SHALL run the normal chain for its class: discovery from `start` when discoverers are registered, otherwise the platform tier; `None` with no start SHALL fall back to platform and SHALL NOT consult the ambient working directory. Every `Some` field SHALL be validated before any gap-filling executes. All-filled input SHALL validate every field and return the bundle without executing any discovery. The output bundle SHALL carry one `ResolvedRoot` per class with per-kind provenance from the tier that supplied it.

#### Scenario: Complete family validates and gaps fill

- **WHEN** a caller supplies complete Configuration and State directories (`~/.config/agentmux-rnd`, `~/.local/state/agentmux-rnd`) with Cache and Data missing
- **THEN** the supplied kinds validate and report `CallerProvided` while Cache and Data resolve through discovery or platform, all in one returned bundle.

#### Scenario: Invalid `Some` fails before gap-filling

- **WHEN** one `Some` value is empty, relative-without-start, or a Windows ambient-dependent shape while other kinds would resolve cleanly
- **THEN** the typed refusal returns before any discovery or platform work runs for any kind.

#### Scenario: Missing kinds never consult ambient state

- **WHEN** a `None` kind resolves with no start directory supplied
- **THEN** the platform tier supplies the result; the process working directory is never read.

#### Scenario: All-filled input is a pure validator

- **WHEN** every kind is supplied complete and valid
- **THEN** the bundle returns with `CallerProvided` provenance throughout and no subprocess, marker search, or filesystem discovery work runs.

#### Scenario: Complete values are used literally

- **WHEN** a `Some` value names a base-shaped directory a caller might have wanted structure appended to
- **THEN** the library uses it literally (validating shape only) and never appends class segments or structure; bases needing composition go through `local_root` first.
