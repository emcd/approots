# Design: opt-in environment overrides

## Context

Every consumer re-implements flag-versus-environment precedence before calling the library: LitRPG's `state_directory` chain (CLI > `LITRPG_STATE_DIRECTORY` > debug config > fallback), Agentmux's flag/env/XDG tiers. The library currently accepts only a single explicit root per resolve, so callers must collapse their own chains and all provenance arrives as `ExplicitDirectory`. The goal is to let the library own the chain for callers that opt in, so LitRPG can delete its caller-side resolution and match on provenance instead.

Constraints carried forward: read-only resolution; no template/env expansion in returned paths; platform-provider blank reads as absent and explicit empties are refused (platform-roots), while declared environment values follow the stricter env-overrides rule (only unset reads as absent; zero-length is malformed); provenance from the same captured inputs as the root (honest adapter); closed vocabulary (any addition needs both tiers).

## Goals / Non-Goals

Goals: per-class opt-in env declarations; explicit > env > discovery > platform precedence with `Some` beating env; a distinct environment provenance; validation identical to the explicit tier; zero behavior change when nothing is declared.

Non-Goals: no app-prefix convention (`{APP}_{CLASS}_DIRECTORY` auto-reading); no child-environment stamping or `GIT_*`-style scrubbing; no per-file lookup or merging; no change to Agentmux's caller-side chains (they stay undeclared); no new dependencies.

## Decisions

1. **Per-class declaration with shape, not app-prefix convention.** A builder call maps one class to one variable name plus whether its value is an exact directory or a layout base. Rationale: nothing is read that the caller did not name, so there is no implicit behavior and no naming convention baked into the library; the shape mirrors the `ExplicitDirectory`/`ExplicitBase` duality, so validation and composition follow the matching explicit shape. Alternative (automatic `{APP}_{CLASS}_DIRECTORY`) was rejected: it widens ambient reads for every caller and would surprise Agentmux's `-rnd`-style deployments.

2. **New provenance pair (`AppvarDirectory`/`AppvarBase`), not explicit-value reuse.** Rationale: the point of the change is letting LitRPG stop resolving CLI-vs-env itself; it must then be able to tell a function argument from an environment value (e.g. source-sensitive policies like hydration gating). The pair mirrors `ExplicitDirectory`/`ExplicitBase` exactly, so the shape distinction survives the channel distinction. Agentmux evidence shows the collapse is safe for them, but they keep resolving caller-side and stay undeclared, so they never observe the new values. Alternative (reuse of the explicit values) was rejected: it would force LitRPG to keep its own chain to recover the distinction, defeating the purpose.

3. **Precedence explicit > env > discovery/development > platform, `Some` beats env.** Rationale: matches every existing consumer chain (CLI > env > fallback), so LitRPG's migration is a deletion, not a reorder. A `PartialRoots::Some` is an explicit-tier value and keeps explicit-tier precedence.

4. **Unset reads as absent; zero-length is an `EmptyPath` error.** Rationale: omissibility is not nullability — only an absent variable falls through, exactly like the explicit tier refuses empty paths. This catches accidental `VAR=` exports (for example from unset shell expansions without `set -u`) loudly instead of silently falling through to another tier, where a state root could land somewhere surprising. A whitespace-only value is non-empty and validates as a relative path. No new validation grammar is introduced.

5. **Capture once per resolve, cached per distinct name.** Rationale: the platform-snapshot lesson (R1) — read the declared variables at resolve start and derive root and provenance from that capture, never re-reading around provider calls. A family resolve captures once for all gap kinds. Each distinct variable name is read exactly once and the cached native value (including absence) is reused for every class declaring that name. Windows may resolve case aliases to the same OS variable, but different declared spellings are separate cache keys and separate reads; no equality guarantee across reads under mutation. One read per name per capture is guaranteed; an atomic process-wide snapshot is explicitly not claimed.

6. **Native reads, resolve-time declaration checks.** Values are read with `var_os` (the `var`/`var_os` distinction in `std::env`: only the `_os` form preserves non-UTF8 bytes), and variable names that are empty or contain `=`/NUL plus repeated per-class declarations are typed refusals (`InvalidEnvDeclaration`, `DuplicateEnvDeclaration`) raised at capture, before any tier resolves. Rationale: declaration errors must surface even when the tier never runs, and lossy conversion would silently retarget a valid non-UTF8 root.

7. **Family barrier extends to environment; arguments still validated.** Eligible gap environment values validate before any gap discovery or platform work (the `Some`-validation barrier, extended), an invalid declared value is never masked by another kind's valid root, and discoverer argument validation runs even when the environment tier wins — only all-filled input bypasses both discovery and argument validation, and it never consults the environment. Rationale: one validate-all-first rule for every caller-controlled input, with a single documented bypass.

## Risks / Trade-offs

- [Vocabulary reopen] The closed five-value set becomes seven; both tiers must approve the addition. → Mitigation: the delta amends the closed-vocabulary requirement explicitly rather than sneaking a value in.
- [Ambient mutability] Process env can change between reads. → Mitigation: single capture per resolve; contract tests use child processes with fixture-owned environments (the nb-api pattern), never mutating the runner's env.
- [Exhaustive-match breakage] Downstream `match` statements over `Provenance` gain a variant and fail to compile until handled. → Mitigation: intended and compiler-surfaced; the crate is pre-1.0 so this rides the 0.4.0 minor, called out in the changelog.

## Migration Plan

LitRPG declares its two variables and deletes its caller-side chain; behavior is identical by construction (same precedence, same validation). Agentmux and Cistella declare nothing: their resolved paths are byte-identical, with provenance labels per the current vocabulary. Rollback is declaration removal. No data migration: resolution creates nothing.

## Open Questions

- None blocking. (`AppvarDirectory`/`AppvarBase` naming follows the operator-settled series.)
