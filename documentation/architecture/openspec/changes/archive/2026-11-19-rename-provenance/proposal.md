## Why

The provenance vocabulary violates the project's nomenclatural standards and leaks one VCS's topology into the generic framework: `DevelopmentCommon`/`DevelopmentWorktree` encode Git's common/worktree distinction, which should have been purged when `VcsDiscovererGit` was split out — no other VCS has a worktree concept, and the generic layer must not carry it. The set also mixes naming schemes with no consistent series. Pre-1.0 breakage is free, so the rename happens now, before more consumers match on the old names. The environment work builds on the renamed vocabulary.

## What Changes

- **BREAKING** Rename the closed provenance set to a consistent source-first series (old → new):
  - `CallerProvided` → `ExplicitDirectory` (exact directory passed by the calling application).
  - `SuppliedDirectory` → `ExplicitBase` (layout scheme derived from an explicit base, e.g. `--prefix`-style roots, `local_root` mating).
  - `DevelopmentCommon`, `DevelopmentWorktree` → `Discovery` (layout scheme derived from a discovered base; anchor selection stays discoverer-internal via the discoverer's `scope` option).
  - `PlatformEnvironment` → `SysvarUserDirectory` (platform per-user root influenced by a system environment variable).
  - `PlatformDefault` → `DefaultUserDirectory` (computed platform per-user default).
- No path, precedence, validation, or discovery behavior changes: resolution returns byte-identical roots, only the labels change.
- `SiteDirectory` values (`SysvarSiteDirectory`, `DefaultSiteDirectory`) are explicitly NOT added — the resolver cannot produce site roots yet, and unproducible normative values are a Tier-2 finding waiting to happen. The names are reserved in the site-wide discovery todo.
- No VCS-specific or topology-specific terms remain in the generic vocabulary; reviewers must verify this (they should have caught it at the `VcsDiscovererGit` split).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `platform-roots`: owns the closed provenance vocabulary — the renamed five-value set, precedence wording, and every scenario naming a value.
- `supplied-resolution`: `ExplicitBase` in the beneath-supplied-directory scenario and the `local_root` requirement.
- `development-layout`: `ExplicitBase` for raw supplied bases, `Discovery` for discovered bases, `SysvarUserDirectory`/`DefaultUserDirectory` for untouched platform results.
- `family-resolution`: `ExplicitDirectory` for `Some` values and all-filled bundles.

## Impact

- `src/types.rs`: the `Provenance` enum and its uses; every exhaustive match in `src/`, `tests/`, and downstream consumers fails to compile until updated (intended, compiler-surfaced).
- Specs and scenarios naming old values, plus the in-flight environment change, which is rebased onto the new vocabulary (`AppvarBase`/`AppvarDirectory`).
- Consumers migrate matches only; resolved paths are unchanged.
