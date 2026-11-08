## ADDED Requirements

### Requirement: Discoverer genera and arguments

The library SHALL organize discoverers by a closed `DiscovererGenus` enum (initially `Vcs` and `Devfile`) and SHALL hold at most one discoverer per genus in a resolver, so the genus unambiguously addresses argument routing. Callers SHALL pass per-genus string arguments (for example `.discoverer_argument(DiscovererGenus::Vcs, "scope", "common")`); each discoverer SHALL deserialize its arguments into its own typed options and SHALL reject unknown keys or values with a typed error naming the discoverer, key, and value. The core interface SHALL NOT define a scope enum: anchor selection is the installed VCS discoverer's own option (for the Git discoverer, `scope` deserializes to `Scope::{Common, Local}`).

#### Scenario: Arguments route by genus

- **WHEN** a caller passes a `scope` argument for `DiscovererGenus::Vcs` while a devfile discoverer is also registered
- **THEN** only the VCS discoverer consumes the argument; any same-named devfile option is unaffected.

#### Scenario: Unknown discoverer argument is a typed error

- **WHEN** a caller passes a key or value the installed discoverer does not define
- **THEN** the library returns a typed error naming the discoverer, key, and value instead of ignoring it.

#### Scenario: Second discoverer in a genus is refused

- **WHEN** a caller registers a second discoverer for a genus that already has one
- **THEN** the library refuses the registration rather than silently replacing or chaining discoverers.

### Requirement: Git discoverer behind the generalized interface

The library SHALL provide `VcsDiscovererGit`, implementing the existing hardened Git discovery contract (explicit working directory, prefix environment scrubbing, typed failure outcomes, identity validation) behind the discoverer interface, returning a single discovered base per discovery. There SHALL be no direct Git entry point beside the discoverer: registering `VcsDiscovererGit` is the caller's signal that Git discovery is wanted. A devfile discoverer SHALL act as both discoverer and validator: given a VCS base it validates that its marker files are present at that base and that the project identity inside matches the application name supplied through the discovery context (or an explicit expected-name argument); without a VCS base it searches upward from the start directory for its marker and proposes the containing directory as the base. An identity mismatch SHALL be a typed error, never a fallthrough.

#### Scenario: Git discoverer yields a single base

- **WHEN** the Git discoverer runs inside a linked worktree with its `scope` option set to `Common`
- **THEN** the discovery returns the common worktree root as the single base, and development resolution anchors there without the core interface naming Git topology.

#### Scenario: Devfile discoverer validates identity at a VCS base

- **WHEN** the Cargo devfile discoverer is given a VCS base whose `Cargo.toml` names a different package than the application name in the discovery context
- **THEN** the library returns a typed identity-mismatch error and does not fall through to another base.
