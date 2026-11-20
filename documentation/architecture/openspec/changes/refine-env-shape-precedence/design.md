# Design: environment shape precedence

## Context

The env-overrides capability lets callers declare one environment variable per directory class, with a shape: exact directory or layout base. The approved contract forbids a second declaration for a class outright, which leaves the natural LitRPG-style pair (`LITRPG_STATE_DIRECTORY` plus a `--prefix`-style base) unrepresentable and the relative order of the two shapes unspecified — an unimplementable gap, since both values cannot win at once.

## Goals / Non-Goals

Goals: both shapes declarable per class; deterministic exact-beats-base order; fail-fast validation of both values. Non-Goals: no change to the tier's position in the chain, no change to capture/masking/barrier mechanics, no new error variants.

## Decisions

1. **One declaration per (class, shape); duplicates are same-shape repeats.** Rationale: the duplicate rule's purpose was ambiguity prevention, and two shapes are unambiguous — shape distinguishes them. Same-shape repeats stay refused.
2. **Exact directory beats layout base when both are usable; both validate regardless.** Rationale: the exact value is more specific (it names the directory; the base names an ancestor scheme), so it wins, and validate-all-first extends naturally within the class: an invalid value of either shape is a typed refusal even when the other shape would win. Alternative (fall back to base when exact is invalid) was rejected: it would mask configuration errors the fail-fast rule exists to catch.
3. **No new error variants, no capture changes.** Rationale: `InvalidEnvDeclaration`/`DuplicateEnvDeclaration` already cover the failure modes; the per-distinct-name cache is unaffected (two declarations for one class may name the same or different variables).

## Risks / Trade-offs

- [Callers relying on the duplicate refusal] Someone catching `DuplicateEnvDeclaration` for mixed-shape pairs gets resolution instead. → Mitigation: pre-1.0-adjacent minor behavior (0.4.0 just shipped the tier; the pair was never usable), called out in the changelog.
- [Double validation cost] Both values validate even when exact wins. → Mitigation: validation is pure and cheap; correctness over skipped work.

## Migration Plan

LitRPG-style callers declare both variables; previously rejected pairs now resolve. No migration for anyone else. Rollback is declaration removal.

## Open Questions

None.
