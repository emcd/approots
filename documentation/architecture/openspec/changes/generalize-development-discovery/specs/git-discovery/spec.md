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

The library SHALL provide `VcsDiscovererGit`, implementing the existing hardened Git discovery contract (explicit working directory, prefix environment scrubbing, typed failure outcomes, identity validation) behind the discoverer interface, returning a single discovered base per discovery. There SHALL be no direct Git entry point beside the discoverer: registering `VcsDiscovererGit` is the caller's signal that Git discovery is wanted. A devfile discoverer SHALL act as both discoverer and validator: given a VCS base it validates that its marker files are present at that base and that the project identity inside matches the application name supplied through the discovery context (or an explicit expected-name argument); with no VCS discoverer registered it searches upward from the start directory for its marker and proposes the containing directory as the base (a registered VCS discoverer reporting genuine absence is not this case: the discovered tier is then absent and no standalone search runs). An identity mismatch SHALL be a typed error, never a fallthrough.

#### Scenario: Git discoverer yields a single base

- **WHEN** the Git discoverer runs inside a linked worktree with its `scope` option set to `Common`
- **THEN** the discovery returns the common worktree root as the single base, and development resolution anchors there without the core interface naming Git topology.

#### Scenario: Devfile discoverer validates identity at a VCS base

- **WHEN** the Cargo devfile discoverer is given a VCS base whose `Cargo.toml` names a different package than the application name in the discovery context
- **THEN** the library returns a typed identity-mismatch error and does not fall through to another base.

## MODIFIED Requirements

### Requirement: Explicit-topology Git discovery

Git topology (`worktree_root`, `common_worktree_root`, `git_common_dir`) SHALL be a private observation inside `VcsDiscovererGit`, used to select the single returned base; the public discovery output is that selected base only, chosen by the discoverer's `scope` option (`Common` selects the common worktree root, `Local` selects the current worktree root, and an omitted `scope` defaults to `Local`). The discoverer SHALL invoke `git rev-parse --show-toplevel` and `--git-common-dir` with an explicit working directory. Relative `--git-common-dir` output SHALL be resolved against the exact subprocess working directory (the explicit start directory the command was invoked from), never against `worktree_root`. `common_worktree_root` SHALL be discovered without assuming `git_common_dir.parent()` is the owner: separate git dirs, submodules, bare common repositories, and missing main worktrees resolve to a supported topology or a typed unsupported outcome. Outputs SHALL use Git's default path format; the library SHALL NOT return `--path-format=absolute` (documented absolute-and-canonical) results as propagated roots, and SHALL distinguish Git's physical-path observations from caller-provided lexical aliases, which it preserves.

#### Scenario: Linked worktree distinguishes roots

- **WHEN** discovery runs inside a linked worktree
- **THEN** the private topology holds distinct worktree and common roots, and the `scope` option selects which one is returned as the base.

#### Scenario: Local state stays worktree-local in a linked worktree

- **WHEN** the Git discoverer runs inside a linked worktree with `scope` set to `Local` (or omitted) and the caller requests the state class
- **THEN** resolution anchors at the current worktree root, never the common worktree path.

#### Scenario: Independent clone collapses roots

- **WHEN** discovery runs in a normal clone or a clone made from another clone
- **THEN** the selected base is the same under either `scope` value.

#### Scenario: Unusable names fall back to generic

- **WHEN** a worktree or project directory name is non-UTF8 or missing
- **THEN** name derivation yields no name and callers fall back to a generic filename instead of failing.

#### Scenario: Nested invocation in a normal clone

- **WHEN** discovery is invoked from a nested subdirectory and `--git-common-dir` returns a relative path
- **THEN** the library resolves it against the invocation directory (yielding the checkout's real `.git`), never against the worktree root.

#### Scenario: Nested invocation in a linked worktree

- **WHEN** discovery is invoked from a nested subdirectory of a linked worktree
- **THEN** the selected base follows the `scope` option between the linked worktree root and its owner checkout root.

#### Scenario: Results are cwd-invariant

- **WHEN** discovery runs from the same explicit start directory under two different process working directories
- **THEN** both runs return identical results, because the library never consults the implicit process working directory.

### Requirement: Optional Cargo identity validation

The Cargo devfile discoverer SHALL operate in exactly one of two modes per resolution. At a selected VCS base, it SHALL validate the marker at that base only: the root `Cargo.toml` at the base, with no upward search beyond the base and no descent into nested member manifests; a missing marker at the selected base, an unreadable or malformed marker, a virtual-workspace manifest, or a `[package].name` that does not match the expected identity (the discovery-context application name, or the explicit expected-name argument when supplied) SHALL each be a typed error, never a fallthrough and never resolution beneath a foreign or wrong project root. Without a VCS discoverer registered, it SHALL search upward from the explicit start directory to the filesystem root, the nearest `Cargo.toml` winning, validating identity at the containing directory it proposes as base; finding no marker SHALL mean the discovered tier is absent (ordinary fallthrough), not an error.

#### Scenario: Foreign checkout is refused

- **WHEN** discovery finds a checkout whose package name does not match the requested marker
- **THEN** the library returns an explicit identity failure and no path.

#### Scenario: Marker walk lands on the workspace root

- **WHEN** identity validation examines a selected VCS base whose root manifest declares the workspace/package
- **THEN** it validates that root manifest, never a nested member manifest.

#### Scenario: Nested or virtual workspace does not escape

- **WHEN** a selected VCS base holds a virtual manifest without a root package, or a proposed standalone base does
- **THEN** the library returns a typed undetermined/virtual outcome and never resolves state or configuration against the wrong project root.

#### Scenario: Missing marker at a selected VCS base is typed

- **WHEN** a VCS base is selected and no `Cargo.toml` exists at that base while the Cargo devfile discoverer is registered
- **THEN** the library returns a typed unreadable/missing-marker error instead of treating the base as validated or falling through.

### Requirement: Discovery state machine

Discovery SHALL proceed by registered genera, in this order, after argument validation. (1) Arguments for every genus SHALL be validated and deserialized before any discoverer executes: an argument bag for a genus with no registered discoverer, an unknown key, or an invalid value SHALL be a typed error at that point, and no discovery work SHALL have run. (2) With no discoverer registered, the discovered tier is absent. (3) With a VCS discoverer registered, it runs at the explicit start: a selected base proceeds to step (4); genuine absence of a repository means the discovered tier is absent and resolution falls through by precedence, and the devfile discoverer SHALL NOT then run a standalone search; executable-unavailable, command-failure, and unsupported-topology outcomes SHALL propagate as typed errors and SHALL NOT fall through to devfile search or platform roots. (4) With a base selected and a devfile discoverer registered, the devfile discoverer validates at that base only, under the Cargo identity requirement's typed outcomes. (5) With only a devfile discoverer registered, it runs its standalone upward search; a validated marker yields the base, and no marker means the discovered tier is absent. Registration of a discoverer is the only opt-in: argument bags never cause a discoverer to be treated as registered, and discoverers never execute for a resolution whose explicit-root tier already decided the result.

#### Scenario: No discoverer means no discovery

- **WHEN** a resolver with no discoverer registered resolves inside a checkout with no higher-precedence supplied root
- **THEN** the platform tier supplies the result and no discoverer executes.

#### Scenario: Git failure never becomes a Cargo or platform fallback

- **WHEN** a registered Git discoverer fails with executable-unavailable, command failure, or unsupported topology
- **THEN** the typed error propagates and neither the Cargo search nor the platform tier is consulted for a discovered result.

#### Scenario: Git absence skips the standalone Cargo search

- **WHEN** a registered Git discoverer reports genuine absence of a repository and a Cargo devfile discoverer is also registered
- **THEN** the discovered tier is absent and resolution falls through by precedence without an upward Cargo search.

#### Scenario: Argument validation precedes discovery

- **WHEN** a caller supplies an argument bag for an unregistered genus, or an invalid value for a registered discoverer, and then resolves
- **THEN** the typed argument error returns before any subprocess, marker search, or filesystem discovery work runs.
