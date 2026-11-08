## ADDED Requirements

### Requirement: Local structure on supplied bases

The library SHALL provide `local_root`, mating a `LocalStructure` onto an explicitly supplied base `ResolvedRoot` and inheriting that base's provenance exactly (for a raw supplied base, `SuppliedDirectory`), so supplied bases receive the same structured layout as discovered bases.

#### Scenario: Parallel instances on distinct supplied bases

- **WHEN** two server instances are each supplied a distinct base and mate the same local structure onto it
- **THEN** each instance resolves its class directories beneath its own base, and neither contends over the platform-provided roots.
