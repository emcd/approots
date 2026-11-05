## ADDED Requirements

### Requirement: Explicit-topology Git discovery

The library SHALL discover Git checkout topology by invoking `git rev-parse --show-toplevel` and `--git-common-dir` with an explicit working directory, returning `worktree_root`, `common_worktree_root`, and `git_common_dir`. Relative `--git-common-dir` output SHALL be resolved against the exact subprocess working directory (the explicit start directory the command was invoked from), never against `worktree_root`. `common_worktree_root` SHALL be discovered without assuming `git_common_dir.parent()` is the owner: separate git dirs, submodules, bare common repositories, and missing main worktrees resolve to a supported topology or a typed unsupported outcome. Outputs SHALL use Git's default path format; the library SHALL NOT return `--path-format=absolute` (documented absolute-and-canonical) results as propagated roots, and SHALL distinguish Git's physical-path observations from caller-provided lexical aliases, which it preserves.

#### Scenario: Linked worktree distinguishes roots

- **WHEN** discovery runs inside a linked worktree
- **THEN** the library returns distinct `worktree_root` and `common_worktree_root` values rooted at the common worktree owner.

#### Scenario: Independent clone collapses roots

- **WHEN** discovery runs in a normal clone or a clone made from another clone
- **THEN** `worktree_root` equals `common_worktree_root`.

#### Scenario: Unusable names fall back to generic

- **WHEN** a worktree or project directory name is non-UTF8 or missing
- **THEN** name derivation yields no name and callers fall back to a generic filename instead of failing.

#### Scenario: Nested invocation in a normal clone

- **WHEN** discovery is invoked from a nested subdirectory and `--git-common-dir` returns a relative path
- **THEN** the library resolves it against the invocation directory (yielding the checkout's real `.git`), never against the worktree root.

#### Scenario: Nested invocation in a linked worktree

- **WHEN** discovery is invoked from a nested subdirectory of a linked worktree
- **THEN** the library returns the linked worktree root and the common worktree root of its owner checkout.

#### Scenario: Results are cwd-invariant

- **WHEN** discovery runs from the same explicit start directory under two different process working directories
- **THEN** both runs return identical results, because the library never consults the implicit process working directory.

### Requirement: Hardened Git execution

The library SHALL scrub ambient Git environment influence using a defined prefix-based policy, treat missing Git and failed commands as explicit outcomes, and define behavior for bare repositories and unusual common-directory layouts without trusting them silently. Prefix scrubbing is not isolation from arbitrary Git executable or configuration behavior: discovery is an observation, never an authorization.

#### Scenario: Contaminated environment does not divert resolution

- **WHEN** hostile `GIT_*` environment variables are present
- **THEN** discovery either still resolves the correct checkout or returns an explicit failure, never a diverted path.

#### Scenario: Application overrides survive scrubbing

- **WHEN** an application-namespaced override family is set alongside hostile `GIT_*` variables
- **THEN** prefix-based scrubbing removes only the `GIT_*` influence and the whole application override family (every variable under the application's own prefix) remains in effect, without the library knowing any application's prefix.

### Requirement: Optional Cargo identity validation

The library SHALL optionally validate a discovered checkout by requiring a matching `[package].name` in its `Cargo.toml`, rejecting foreign, mismatched, and virtual-workspace checkouts explicitly instead of resolving beneath them. Marker search SHALL stay bounded to the intended checkout and never search outside it.

#### Scenario: Foreign checkout is refused

- **WHEN** discovery finds a checkout whose package name does not match the requested marker
- **THEN** the library returns an explicit identity failure and no path.

#### Scenario: Marker walk lands on the workspace root

- **WHEN** identity validation walks for a `Cargo.toml` package marker
- **THEN** it lands on the workspace root matching `git --show-toplevel` behavior, never on a nested member manifest.

#### Scenario: Nested or virtual workspace does not escape

- **WHEN** discovery finds a nested workspace member or a virtual manifest without a root package
- **THEN** the library validates workspace boundaries and never resolves state or configuration against the wrong project root.

### Requirement: Lexically-contained joins with typed ambiguity

The library SHALL check every joined segment against a token grammar (non-empty; no native separator; no Windows prefix or alternate-data-stream colon; no NUL; never `.` or `..`) before joining, and SHALL report ambiguity from canonical comparison as a typed error rather than silently picking the first match. Supplied full roots stay distinct from joined single-token names. Canonical comparison resolves symlinks for detection only; its result SHALL NOT be returned or propagated as a resolved path.

#### Scenario: Crafted segment cannot escape its slot

- **WHEN** a caller-supplied name contains a separator or traversal-only value
- **THEN** the library rejects it before any path join.

#### Scenario: Symlink-collapsed candidates are an error

- **WHEN** canonical comparison collapses two candidate directories to the same target
- **THEN** the library returns a typed ambiguity error instead of resolving to either.
