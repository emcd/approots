## 1. Method

- [ ] 1.1 Add `ResolvedRoot::join` with per-segment token validation (validate-all-first) and provenance inheritance.
- [ ] 1.2 Pin empty-list identity and absolute/empty-segment refusals.

## 2. Contract tests

- [ ] 2.1 Pin multi-segment joins (Cistella shapes: `profiles`, `logs/<op-id>`, `in-container`) with provenance preserved.
- [ ] 2.2 Pin invalid segments (separator, traversal, Windows shape, empty, NUL) with the offending segment named.
- [ ] 2.3 Pin literal use: segments are appended, never conditionally composed; joining onto any tier's output behaves identically.

## 3. Implementation

- [ ] 3.1 Implement the method and contract tests; verify gates (nextest, clippy, openspec, cross-target checks).
