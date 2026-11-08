# development-layout

## Purpose

Common-worktree and worktree-local `.auxiliary` candidates with explicit `Common` vs `Worktree` scope selection and caller-overridable layout. Owns layout and scope; delegates layer-list mechanics to `platform-roots`.

## Requirements

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

### Requirement: Directory classes

The library SHALL name the four directory classes `Configuration`, `State`, `Cache`, and `Data` through the `DirectoryClass` type, consistent with the `distinct_directory_classes` option vocabulary.

#### Scenario: Class vocabulary is uniform

- **WHEN** a caller names a class for any resolution tier
- **THEN** the same `DirectoryClass` type and segment vocabulary apply across supplied, discovered, layered, and platform resolution.

### Requirement: Exact local_root composition and provenance

`local_root` SHALL compose exactly `<base>/<prefix>/<class-segment>/<application>` — with the default structure, prefix `.auxiliary` and class segments `configuration`, `state`, `cache`, `data` — appending the application segment exactly once to the given base, and SHALL inherit the provenance of the `ResolvedRoot` supplied as its base rather than inventing one. Bases SHALL be absolute and non-empty, and on Windows SHALL NOT use ambient drive-relative or root-relative shapes: with no explicit-start parameter, unresolved bases are typed refusals — `EmptyPath` for empty, `RelativeStartDirectory` for relative, `InvalidName` for Windows ambient-dependent shapes — never propagated relative roots. Valid bases are propagated lexically and SHALL NOT be canonicalized. Rawness (not already application-namespaced) SHALL remain an unchecked caller precondition: `local_root` composes onto whatever valid base it is given, including a raw base that itself ends in the application name. A `platform_root` result is a complete root, not a `local_root` base, and `Resolver` SHALL never feed its completed platform-tier result into `local_root`. In `Resolver`, only the discovered and supplied tiers apply the local structure; the platform tier SHALL NOT have the structure applied to it (preserving the platform contract is a non-goal constraint on this change), and `distinct_directory_classes` SHALL affect only platform-tier results, never `local_root` compositions. The application namespace guarantees (validated literal name, no cross-call uniqueness promise) SHALL apply unchanged.

#### Scenario: Supplied base example

- **WHEN** a caller mates the default structure onto an absolute supplied base (POSIX example `/srv/inst1`; any absolute base on the host platform) for the state class of application `litrpg`
- **THEN** the result is that base joined with `.auxiliary/state/litrpg` (`/srv/inst1/.auxiliary/state/litrpg` on POSIX) with `SuppliedDirectory` provenance.

#### Scenario: Discovered Git examples

- **WHEN** the Git discoverer selects the common root `/repo` (`scope` = `Common`) for configuration, or the worktree root `/repo-wt` (`scope` = `Local`) for state (POSIX examples; any absolute discovered base on the host platform)
- **THEN** the results are those bases joined with `.auxiliary/configuration/<application>` and `.auxiliary/state/<application>` respectively, with `DevelopmentCommon` and `DevelopmentWorktree` provenance; a Cargo-only discovered base likewise carries `DevelopmentWorktree` as the local-anchor value of the closed provenance set.

#### Scenario: Platform root is not a local_root base

- **WHEN** a caller holds a `platform_root` result (already ending in the application name)
- **THEN** that root is used as a complete root; `Resolver` never feeds its completed platform-tier result into `local_root`, and platform-tier results keep their exact platform paths and `PlatformEnvironment`/`PlatformDefault` provenance.

#### Scenario: Raw base ending in the application name still composes

- **WHEN** a caller mates the default structure onto an absolute raw base ending in the application name (POSIX example `/repo/litrpg`) for the state class of application `litrpg`
- **THEN** the result is that base joined with `.auxiliary/state/litrpg` (`/repo/litrpg/.auxiliary/state/litrpg` on POSIX): rawness is a caller precondition the library does not check, and composition appends the application segment exactly once relative to the given base.

#### Scenario: Unresolved bases are refused

- **WHEN** a caller mates the structure onto an empty, relative, or Windows ambient-dependent base
- **THEN** the library returns `EmptyPath`, `RelativeStartDirectory`, or `InvalidName` respectively instead of a propagated relative root.

### Requirement: Layered roots supersede the overrides convention

The library SHALL resolve ordered closed directory roots (an empty layer list is rejected) and SHALL NOT define an `overrides/` subdirectory convention. Per-file lookup, existence checks, and fault policy within those roots belong to consumers; the library never turns a permission or query error into absence via boolean existence probes.

#### Scenario: Consumer lookup finds the earlier file

- **WHEN** a consumer looks up the same relative file within the resolved ordered roots and both an earlier and a later root hold it
- **THEN** the earlier (local) root is where the consumer finds it, because the library returned the roots in precedence order.

### Requirement: Caller-supplied application namespace

The library SHALL namespace every development root by a caller-supplied application name and never hardcode a per-deployment suffix.

#### Scenario: Validated names resolve literally

- **WHEN** callers request development roots for validated application names from the same checkout
- **THEN** each name resolves literally to its own namespaced tree; uniqueness across calls or applications is the caller's responsibility through distinct validated names, never a library promise.

### Requirement: Explicit development activation

The library SHALL resolve discovered development candidates only when the caller has explicitly registered a discoverer; a resolver with no discoverer registered never discovers. Otherwise, when no higher-precedence supplied root exists, it MUST fall through to platform-standard roots.

#### Scenario: Release caller does not adopt a checkout

- **WHEN** no discoverer is registered, no higher-precedence supplied root exists, and the process runs inside a checkout
- **THEN** the library returns the platform-standard root and never a development-tree path.
