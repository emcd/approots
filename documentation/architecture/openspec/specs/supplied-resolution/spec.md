# supplied-resolution

## Purpose

Conventional config/state/cache/data locations relative to an explicit supplied directory, no repository required. Owns the cross-cutting resolution invariants (read-only behavior, literal paths, relative-path base).

## Requirements

### Requirement: Supplied-directory kind resolution

The library SHALL resolve configuration, state, cache, and data locations relative to an explicitly caller-supplied directory without requiring a repository or marker.

#### Scenario: Resolve beneath supplied directory

- **WHEN** a caller supplies a directory and requests the configuration kind
- **THEN** the library returns the conventional configuration location beneath that directory with `SuppliedDirectory` provenance and creates nothing.

#### Scenario: Non-repository directory is accepted

- **WHEN** the supplied directory is not a Git checkout
- **THEN** resolution still succeeds for all kinds without attempting repository discovery.

### Requirement: Returned paths are literal

The library SHALL return paths literally: no template or environment expansion, and the returned path SHALL NOT be the result of symlink-following canonicalization. (Comparison for ambiguity detection MAY resolve symlinks; its result SHALL NOT be returned or propagated.) Returned paths are candidates, not trusted anchors: symlink targets may change after resolution, and the library gives no guarantee they cannot.

#### Scenario: Spans pass through unresolved

- **WHEN** a supplied directory or application name contains template-like spans
- **THEN** the library returns them unresolved and never expands them.

### Requirement: Stated relative-path base

The caller-supplied explicit start directory SHALL be absolute. Ordinary relative inputs SHALL be joined against it before any absolute handling, never resolved through the ambient process working directory (which `std::path::absolute` would consult). Windows drive-relative and root-relative inputs that would consult ambient drive state SHALL be refused. (Consumers that today rely on cwd-based resolution pass their working directory as the explicit start directory and observe no change.)

#### Scenario: Relative explicit path keeps its meaning

- **WHEN** a caller supplies a relative explicit path together with an explicit start directory
- **THEN** it resolves against that start directory, and changing the process working directory never changes what it names.

#### Scenario: Local R&D tree resolves deterministically

- **WHEN** a caller supplies a relative local-tree base for an R&D bundle root from two different process working directories
- **THEN** both resolutions name the same directory.

### Requirement: Read-only resolution

The library SHALL NOT create, modify, delete, or lock anything on the filesystem, SHALL NOT parse or merge consumer application configuration, and SHALL NOT enforce ownership or permissions on the paths it returns. (Cargo.toml identity parsing is the sole declared exception.) The library MAY read and stat what it needs to resolve (including treating an unreadable layer as a reportable fault).

#### Scenario: Resolution touches nothing

- **WHEN** any resolution runs (supplied, discovered, development, or platform)
- **THEN** the filesystem is byte-identical afterwards: no directories created, no files written, no locks taken, no permissions changed.

### Requirement: No silent ancestor search

The library SHALL NOT search ancestor trees of the supplied directory unless the caller explicitly enables discovery.

#### Scenario: Sibling checkout is not adopted

- **WHEN** a caller supplies a directory nested inside an unrelated checkout without enabling discovery
- **THEN** the library resolves strictly beneath the supplied directory and never returns a path rooted at an ancestor checkout.

### Requirement: Local structure on supplied bases

The library SHALL provide `local_root`, mating a `LocalStructure` onto an explicitly supplied base `ResolvedRoot` and inheriting that base's provenance exactly (for a raw supplied base, `SuppliedDirectory`), so supplied bases receive the same structured layout as discovered bases.

#### Scenario: Parallel instances on distinct supplied bases

- **WHEN** two server instances are each supplied a distinct base and mate the same local structure onto it
- **THEN** each instance resolves its class directories beneath its own base, and neither contends over the platform-provided roots.
