## Why

Agentmux and LitRPG independently resolve application directories with Git worktree awareness, and the logic has already drifted in security-relevant ways. A shared, read-only policy crate prevents further reinvention.

## What Changes

- Add a small, caller-driven, read-only resolution library (`approots`).
- Resolve configuration, state, cache, and data roots relative to an explicit caller-supplied directory.
- Add optional hardened Git checkout and linked-worktree discovery (`worktree_root`, `common_worktree_root`, `git_common_dir`).
- Add optional development-tree identity validation (Cargo package marker first).
- Expose common-worktree and worktree-local `.auxiliary` candidates without creating them.
- Report explicit precedence and path provenance for every resolved root.
- Wrap an established platform-directory primitive for platform-standard roots.

## Capabilities

### New Capabilities

- `supplied-resolution`: conventional config/state/cache/data locations relative to an explicit supplied directory, no repository required. Owns the cross-cutting resolution invariants (read-only behavior, literal paths, relative-path base).
- `git-discovery`: hardened Git checkout and linked-worktree discovery with explicit working directory, environment scrubbing, identity validation, and testable failure outcomes (including name-derivation fallback to generic names).
- `development-layout`: common-worktree and worktree-local `.auxiliary` candidates with explicit `Common` vs `Worktree` scope selection and caller-overridable layout. Owns layout and scope; delegates layer-list mechanics to `platform-roots`.
- `platform-roots`: platform-standard roots (XDG/macOS/Windows via a lower-level crate), the closed provenance vocabulary, layer-list mechanics (closed lists, fault-on-unreadable), and shared name sanitization.

### Modified Capabilities

- None. No existing specs.

## Impact

New crate API only. No changes to Agentmux or LitRPG in this change. Sequence: provisional API, then Agentmux migration, then LitRPG migration, then stabilization. Consumers keep ownership of parsing, merging, creation, permissions, locking, and credentials.
