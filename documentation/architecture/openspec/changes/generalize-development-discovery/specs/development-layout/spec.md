## MODIFIED Requirements

### Requirement: Explicit-scope development candidates

The library SHALL mate a caller-overridable `LocalStructure` — a validated segment scheme whose `.auxiliary/{configuration,state}` form is the default — onto any base: a discovered root, a platform root, or a caller-supplied base, returning the base's provenance and creating nothing. Structure overrides are validated relative component sequences beneath the selected base, rejecting rooted, prefixed, or traversal components; replacing the base itself goes through the explicit-root tier, never the structure override. Which discovered root (shared or local to the checkout) anchors a development candidate is a discoverer option, not a core scope enum.

#### Scenario: Escape-shaped structure override is refused

- **WHEN** a caller supplies a structure override that is rooted, carries a drive or prefix, or contains traversal components
- **THEN** the library rejects it before any join.

#### Scenario: Structure mates to a supplied base

- **WHEN** a caller mates the default local structure onto a supplied per-instance base (for example a `-rnd` state base outside any checkout) and requests the state class
- **THEN** the library returns the structured path beneath that base with the base's provenance, so parallel instances with distinct supplied bases share one structure without contending over the platform-provided roots.

#### Scenario: Shared configuration resolves under the shared discovered root

- **WHEN** a caller requests shared configuration and the installed VCS discoverer's anchor option selects the shared root
- **THEN** the library returns the structured configuration path beneath the shared discovered root with the corresponding development provenance.

### Requirement: Explicit development activation

The library SHALL resolve discovered development candidates only when the caller has explicitly registered a discoverer; a resolver with no discoverer registered never discovers. Otherwise, when no higher-precedence supplied root exists, it MUST fall through to platform-standard roots.

#### Scenario: Release caller does not adopt a checkout

- **WHEN** no discoverer is registered, no higher-precedence supplied root exists, and the process runs inside a checkout
- **THEN** the library returns the platform-standard root and never a development-tree path.

## ADDED Requirements

### Requirement: Directory classes

The library SHALL name the four directory classes `Configuration`, `State`, `Cache`, and `Data` through the `DirectoryClass` type, consistent with the `distinct_directory_classes` option vocabulary.

#### Scenario: Class vocabulary is uniform

- **WHEN** a caller names a class for any resolution tier
- **THEN** the same `DirectoryClass` type and segment vocabulary apply across supplied, discovered, layered, and platform resolution.

### Requirement: Exact local_root composition and provenance

`local_root` SHALL compose exactly `<base>/<prefix>/<class-segment>/<application>` — with the default structure, prefix `.auxiliary` and class segments `configuration`, `state`, `cache`, `data` — appending the application segment exactly once to the given base, and SHALL inherit the provenance of the `ResolvedRoot` supplied as its base rather than inventing one. Bases SHALL be raw (not already application-namespaced) as a caller precondition, and the library does not check this: `local_root` composes onto whatever base it is given, including a raw base that itself ends in the application name. A `platform_root` result is a complete root, not a `local_root` base, and `Resolver` SHALL never feed its completed platform-tier result into `local_root`. In `Resolver`, only the discovered and supplied tiers apply the local structure; the platform tier SHALL NOT have the structure applied to it (preserving the platform contract is a non-goal constraint on this change), and `distinct_directory_classes` SHALL affect only platform-tier results, never `local_root` compositions. Relative bases SHALL be resolved only through the stated relative-path base rules, and the application namespace guarantees (validated literal name, no cross-call uniqueness promise) SHALL apply unchanged.

#### Scenario: Supplied base example

- **WHEN** a caller mates the default structure onto a supplied base `/srv/inst1` for the state class of application `litrpg`
- **THEN** the result is `/srv/inst1/.auxiliary/state/litrpg` with `SuppliedDirectory` provenance.

#### Scenario: Discovered Git examples

- **WHEN** the Git discoverer selects the common root `/repo` (`scope` = `Common`) for configuration, or the worktree root `/repo-wt` (`scope` = `Local`) for state
- **THEN** the results are `/repo/.auxiliary/configuration/<application>` with `DevelopmentCommon` provenance and `/repo-wt/.auxiliary/state/<application>` with `DevelopmentWorktree` provenance respectively; a Cargo-only discovered base likewise carries `DevelopmentWorktree` as the local-anchor value of the closed provenance set.

#### Scenario: Platform root is not a local_root base

- **WHEN** a caller holds a `platform_root` result (already ending in the application name)
- **THEN** that root is used as a complete root; `Resolver` never feeds its completed platform-tier result into `local_root`, and platform-tier results keep their exact platform paths and `PlatformEnvironment`/`PlatformDefault` provenance.

#### Scenario: Raw base ending in the application name still composes

- **WHEN** a caller mates the default structure onto a raw base `/repo/litrpg` for the state class of application `litrpg`
- **THEN** the result is `/repo/litrpg/.auxiliary/state/litrpg`: rawness is a caller precondition the library does not check, and composition appends the application segment exactly once relative to the given base.
