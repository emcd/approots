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

The library SHALL provide `VcsDiscovererGit`, implementing the existing hardened Git discovery contract (explicit working directory, prefix environment scrubbing, typed failure outcomes, identity validation) behind the discoverer interface, projecting its topology onto a generic discovered base: a primary root plus an optional shared root. The direct Git entry point SHALL be named `find_checkout` and SHALL keep returning the full `GitCheckout` topology for callers who want Git explicitly.

#### Scenario: Git discoverer yields a generic base

- **WHEN** the Git discoverer runs inside a linked worktree with its `scope` option set to `Common`
- **THEN** the discovered base's shared root is the common worktree root, and development resolution anchors there without the core interface naming Git topology.
