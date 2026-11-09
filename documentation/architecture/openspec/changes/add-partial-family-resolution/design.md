# Design: family partial-explicit resolution

## Context

`Resolver::resolve` settles one class per call through the precedence
chain (explicit full root, then discovery from an explicit start, then
platform). Consumers resolve families: today's `--*-directory` flags
already carry complete per-kind directories
(`~/.config/agentmux-rnd` for Configuration,
`~/.local/state/agentmux-rnd` for State), and callers loop per kind to
validate those while filling the remaining kinds through discovery or
platform defaults. The bundle shape does not exist yet; neither does a
call that validates a caller-packaged family and completes it.

Stakeholders: LitRPG and Agentmux migrations (first consumers),
Tier-1/Tier-2 reviewers (contract approval before implementation).

## Goals / Non-Goals

Goals:

- One call resolves a whole family with per-kind complete-or-missing
  semantics.
- `Some` handling is byte-for-byte today's explicit tier: same shape
  validation, same `CallerProvided` provenance, same relative-join
  rule. Today's command lines keep working unchanged.
- `None` handling reuses the existing chain untouched (no new
  discovery or platform behavior).
- All-filled input is a pure validator.

Non-Goals:

- No base-to-complete derivation in this change: `Some` fields are
  complete directories, never bases needing class appendages. Base
  mating stays with `local_root` / `supplied_root`.
- No CWD sniffing: `None` with no start falls back to platform, as
  today. The family method does not invent an origin.
- No per-kind discoverer selection or layer-list changes.
- No breaking changes to existing APIs.

## Decisions

**Separate method, not an extended `resolve`.**
`resolve_partially_explicit(partial, start)` keeps the per-call
contract obvious and leaves `resolve` untouched. Alternative
considered: overloading `resolve` with an enum — rejected, it would
complicate the hot path's signature for no behavioral gain.

**New structs rather than reusing `ResolvedRoot` per field.**
Input `PartialRoots` holds `Option<PathBuf>` per class; output holds a
`ResolvedRoot` per class (name to be finalized in review; working
title `ResolvedRoots`). Alternative considered: a `HashMap`-keyed
bundle — rejected, the closed four-class set deserves named fields
that the compiler checks exhaustively.

**Validate-all-Somes before any gap-filling.**
Mirrors validate-before-discovery: an invalid explicit must surface
even when every other kind would resolve cleanly. A mutant that fills
gaps first must fail the ordering control (test required).

**Relative `Some` values follow the explicit-tier join rule.**
Absolute is the common case (shells expand `~`); a relative value
joins against `start`, and relative with no start is a typed error —
identical to `resolve`. No new relative-path semantics.

**Provenance is per kind, from the tier that supplied it.**
`Some` fields report `CallerProvided`; filled fields report whatever
their tier reports. No family-level provenance is invented.

## Risks / Trade-offs

- [Risk] Struct-name confusion (`ResolvedRoots` vs `ResolvedRoot`) →
  Mitigation: flag naming explicitly for Tier-1 review; alternatives
  are `FamilyRoots` / `RootSet`.
- [Risk] Callers pass bases in `Some` fields expecting appendages →
  Mitigation: document that `Some` means complete; bases go through
  `local_root` first. A misuse probe test pins the distinction.
- [Risk] Scope growth toward family-level discoverer options →
  Mitigation: per-kind behavior is delegated, never reimplemented;
  the method owns only validation, ordering, and bundling.

## Migration Plan

Additive only: implement structs + method, pin with ordering and
completion tests, release in 0.2.0. No consumer changes required;
LitRPG/Agentmux adopt opportunistically by replacing per-kind loops.
Rollback is deletion of the new API.

## Open Questions

- Final struct names (`ResolvedRoots` vs alternatives).
- Should the output bundle offer per-kind accessors beyond public
  fields? (Default: plain public fields, matching `ResolvedRoot`.)
