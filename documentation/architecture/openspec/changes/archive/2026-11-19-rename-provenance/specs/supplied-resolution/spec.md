## MODIFIED Requirements

### Requirement: Supplied-directory kind resolution

The library SHALL resolve configuration, state, cache, and data locations relative to an explicitly caller-supplied directory without requiring a repository or marker.

#### Scenario: Resolve beneath supplied directory

- **WHEN** a caller supplies a directory and requests the configuration kind
- **THEN** the library returns the conventional configuration location beneath that directory with `ExplicitBase` provenance and creates nothing.

#### Scenario: Non-repository directory is accepted

- **WHEN** the supplied directory is not a Git checkout
- **THEN** resolution still succeeds for all kinds without attempting repository discovery.

### Requirement: Local structure on supplied bases

The library SHALL provide `local_root`, mating a `LocalStructure` onto an explicitly supplied base `ResolvedRoot` and inheriting that base's provenance exactly (for a raw supplied base, `ExplicitBase`), so supplied bases receive the same structured layout as discovered bases.

#### Scenario: Parallel instances on distinct bases

- **WHEN** two server instances are each supplied a distinct base and mate the same local structure onto it
- **THEN** each instance resolves its class directories beneath its own base, and neither contends over the platform-provided roots.

#### Scenario: Supplied base keeps its renamed provenance

- **WHEN** a caller mates the default structure onto an absolute supplied base
- **THEN** the composed root reports `ExplicitBase` provenance, never a development or platform value.
