# Design: validated appendages

## Context

`ResolvedRoot` is currently terminal: callers that need a subdirectory
beneath it (`<state>/logs/<op>`, `<config>/profiles`) join paths
themselves, outside the crate's validation. The token grammar
(`validate_token`: non-empty, no separators, no drive/prefix colon, no
NUL, no `.`/`..`) already encodes exactly the rules such segments must
satisfy. Stakeholders: Cistella migration (first consumer), Tier-1/Tier-2
reviewers (contract approval).

## Goals / Non-Goals

Goals:

- One validated join covering every tier's output uniformly.
- Segment validation identical to the existing token rule — no new
  grammar.
- Provenance passes through untouched; the derived path is the same
  root, refined.

Non-Goals:

- No discovery, precedence, or platform behavior: the input is already
  resolved.
- No absolute segments, no empty list error: an empty segment list is
  the identity (returns the root unchanged).
- No new directory classes; no per-kind special-casing.

## Decisions

**Method on `ResolvedRoot`, not parameters on `resolve`.**
`join(&self, segments)` works on any tier's output including family
bundles' fields, with zero precedence interaction. Alternatives
considered: `appendages` args on `resolve`/`resolve_partially_explicit`
— rejected, it tangles derivation with resolution for no gain, and
per-kind appendages in the family call would need a parallel struct
for marginal benefit. Revisit only if a consumer shows a hot loop that
needs single-call derivation.

**Reuse `validate_token` per segment, fail on first invalid.**
Same rule as application names and joined single tokens; error names
the offending segment (`InvalidName`). Multi-segment paths join left
to right after all segments validate (validate-all-first, matching the
crate's ordering discipline).

**Absolute or empty segments are refused, empty list is identity.**
A segment containing a separator is already refused by the token rule;
an empty list returns `self.clone()`. Rationale: joining nothing is
not an error, it is a no-op — and refusing it would force callers to
branch on list length.

**Provenance inherited, never invented.**
The derived root reports the base root's provenance; no new variant,
no derivation marker.

## Risks / Trade-offs

- [Risk] Callers pass base-shaped values expecting composition (the
  family-spec misuse case) → Mitigation: `join` appends, never
  conditionally composes; misuse probe pins literal segments.
- [Risk] Method name collides with `Path::join` expectations (which
  replaces on absolute input) → Mitigation: absolute inputs are
  refused by validation, so replacement semantics can never trigger;
  document the distinction in rustdoc.
- [Risk] Scope growth toward per-kind family appendages →
  Mitigation: declined in this change; revisit on consumer evidence.

## Migration Plan

Additive only: one method plus tests, released in 0.3.0. No consumer
changes required; Cistella adopts for its config/cache/state subpaths.

## Open Questions

- Method name (`join` vs `append` vs `with_segments`) — Tier-1 call.
- Should the empty list be identity or a typed refusal? (Default:
  identity, per above; cheap to flip in review.)
