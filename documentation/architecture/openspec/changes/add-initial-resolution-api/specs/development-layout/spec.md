## ADDED Requirements

### Requirement: Explicit-scope development candidates

The library SHALL expose `.auxiliary` development candidates for an explicit `DevelopmentScope` (`Common` vs `Worktree`) without creating any directory. The `.auxiliary/{configuration,state}` segment layout is the DEFAULT and SHALL be caller-overridable. Layout overrides are validated relative component sequences beneath the selected root, rejecting rooted, prefixed, or traversal components; replacing the root itself goes through the explicit-root tier, never the layout override.

#### Scenario: Escape-shaped layout override is refused

- **WHEN** a caller supplies a layout override that is rooted, carries a drive or prefix, or contains traversal components
- **THEN** the library rejects it before any join.

#### Scenario: Shared configuration resolves under common worktree

- **WHEN** a caller requests shared configuration with `DevelopmentScope::Common`
- **THEN** the library returns `<common-worktree>/.auxiliary/configuration/<application>` with `DevelopmentCommon` provenance.

#### Scenario: Worktree-local state stays worktree-local

- **WHEN** a caller requests state with `DevelopmentScope::Worktree` inside a linked worktree
- **THEN** the library returns `<worktree>/.auxiliary/state/<application>` with `DevelopmentWorktree` provenance, never the common worktree path.

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

The library SHALL resolve development candidates only when the caller explicitly enables development mode; otherwise, when no higher-precedence supplied root exists, it MUST fall through to platform-standard roots.

#### Scenario: Release caller does not adopt a checkout

- **WHEN** development mode is disabled, no higher-precedence supplied root exists, and the process runs inside a checkout
- **THEN** the library returns the platform-standard root and never a `.auxiliary` path.
