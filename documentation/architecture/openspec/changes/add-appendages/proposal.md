## Why

Consumers derive subpaths beneath resolved roots — Cistella's `profiles/` under config, `logs/<op-id>/` under state, `guest-cache/` and `in-container/` under cache — and today each would hand-roll the join, reintroducing the exact validation drift approots exists to centralize (separators, traversal, Windows shapes, empty segments). A validated join on the resolved value keeps one code path for subpath derivation with the crate's token grammar enforced.

## What Changes

- Add `ResolvedRoot::join` (working name; Tier-1 may rename): appends caller-supplied path segments to a resolved root, validating each segment with the token grammar and inheriting provenance unchanged. Empty segment list returns the root unchanged.
- Multi-segment paths supported (`logs/<op-id>` joins two levels in one call).
- Pure function on an already-resolved value: no discovery, precedence, or platform interaction.

## Capabilities

### New Capabilities

- `appendages`: validated subpath derivation on resolved roots.

### Modified Capabilities

(none — token grammar, provenance, and path-shape rules are reused unchanged)

## Impact

- Additive public API only (one method); no existing API changes, no dependencies.
- Intended first consumer: Cistella migration (config/cache/state subpaths), unblocking their GC-dispatch work.
- 0.3.0 vehicle, no new directory classes (deferred: confirmed consumer shapes are all subpaths of existing classes).
