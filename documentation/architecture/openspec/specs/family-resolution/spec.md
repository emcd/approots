# family-resolution

## Purpose

Whole-family resolution with per-kind complete-or-missing roots: one call validates a caller-packaged family and fills its gaps through the normal chain.

## Requirements

### Requirement: Family partial-explicit resolution

The library SHALL provide family input/output structs and a `Resolver::resolve_partially_explicit` method settling all four directory classes in one call. Resolution proceeds in ordered phases: (1) validate every `Some` field exactly like the explicit tier — absolute and non-empty, refusing Windows ambient-dependent shapes, joining a relative value against `start` (typed error when relative with no start); (2) if all four kinds are supplied, return the bundle with `ExplicitDirectory` provenance throughout without consulting the environment at all and without executing any discovery; (3) determine the eligible gap set (the `None` kinds) and run declaration checks plus capture for those gap kinds only — declarations for `Some` kinds are never captured, so a malformed value or declaration for a supplied kind stays silent; (4) validate every eligible gap environment value, of every declared shape, before any gap discovery or platform work runs — an invalid declared value is a typed refusal even when earlier gaps would resolve cleanly, and no valid explicit or `Some` root masks another kind's malformed environment; when both shapes are declared for one class, the exact directory wins after both validate; (5) validate discoverer arguments even when the environment tier will supply a gap root; (6) fill each gap in precedence order, reusing one capture with a single read per distinct variable name. A `None` value with no `start` SHALL use the environment tier when a declared variable for that class holds an absolute value needing no `start`; when no declared variable for the kind is set it SHALL fall back to platform; when a declared value is zero-length, relative, or otherwise malformed the typed refusal returns instead of any fallback. The ambient working directory is never consulted. The output bundle SHALL carry one `ResolvedRoot` per class with per-kind provenance from the tier that supplied it.

#### Scenario: Complete family validates and gaps fill

- **WHEN** a caller supplies complete Configuration and State directories (`~/.config/agentmux-rnd`, `~/.local/state/agentmux-rnd`) with Cache and Data missing
- **THEN** the supplied kinds validate and report `ExplicitDirectory` while Cache and Data resolve through discovery or platform, all in one returned bundle.

#### Scenario: Gap fills from declared environment

- **WHEN** a `None` kind declares an exact directory holding a valid absolute value, and all other values declared for eligible gaps, including any base for this same gap, are valid or absent (supplied kinds mask their declarations entirely)
- **THEN** the bundle carries that exact directory with `AppvarDirectory` provenance for the kind. Malformed values in any eligible shape fail first.

#### Scenario: Gap fills from environment without start

- **WHEN** no `start` is supplied, a `None` kind declares an exact directory holding a valid absolute value, and all other values declared for eligible gaps, including any base for this same gap, are valid or absent
- **THEN** the environment tier supplies that exact directory with `AppvarDirectory` provenance instead of the platform fallback. Malformed values in any eligible shape fail first.

#### Scenario: Family gap prefers exact directory over layout base

- **WHEN** a gap kind declares both an exact directory and a layout base holding valid absolute values, and all other values declared for eligible gaps are valid or absent (supplied kinds mask their declarations entirely)
- **THEN** the bundle carries the exact directory with `AppvarDirectory` provenance for the kind.

#### Scenario: Malformed relative environment with no start is an error

- **WHEN** no `start` is supplied and a gap kind's declared variable holds a relative value
- **THEN** the typed refusal returns instead of the platform fallback.

#### Scenario: Invalid environment fails before gap discovery

- **WHEN** a gap kind's declared variable holds a relative value with no `start` while an earlier gap would discover cleanly
- **THEN** the typed refusal returns before any gap discovery or platform work runs for any kind.

#### Scenario: Valid roots never mask another kind's malformed environment

- **WHEN** Configuration is supplied complete while the Cache gap's declared variable is malformed
- **THEN** the typed refusal returns instead of a bundle with a resolved Configuration.

#### Scenario: Invalid `Some` fails before gap-filling

- **WHEN** one `Some` value is empty, relative-without-start, or a Windows ambient-dependent shape while other kinds would resolve cleanly
- **THEN** the typed refusal returns before any environment capture, gap discovery, or platform work runs for any kind.

#### Scenario: Missing kinds never consult ambient state

- **WHEN** a `None` kind resolves with no start directory supplied and none of its declared variables is set
- **THEN** the platform tier supplies the result; the process working directory is never read.

#### Scenario: All-filled input is a pure validator

- **WHEN** every kind is supplied complete and valid (even with declared variables holding values, valid or not)
- **THEN** the bundle returns with `ExplicitDirectory` provenance throughout, the environment is never captured or consulted, and no subprocess, marker search, or filesystem discovery work runs.

#### Scenario: Argument validation runs when environment wins

- **WHEN** a registered discoverer's argument bag is invalid and a gap kind's declared variable holds a valid directory
- **THEN** the validator's typed refusal returns instead of the environment root.

#### Scenario: Complete values are used literally

- **WHEN** a `Some` value names a base-shaped directory a caller might have wanted structure appended to
- **THEN** the library uses it literally (validating shape only) and never appends class segments or structure; bases needing composition go through `local_root` first.
