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
