## Why

Downstream projects each hand-roll the same CLI-flag-versus-environment-variable precedence (LitRPG's `state_directory` chain, Agentmux's flag/env/XDG tiers) before Approots ever sees a path. That duplicated logic is exactly the drift the crate exists to absorb: every caller re-implements validation, blank-handling, and precedence, and each implementation can disagree. An opt-in, per-class environment declaration lets callers hand the whole chain to the library, so LitRPG can delete its caller-side resolution and match on provenance instead of re-deriving it.

## What Changes

- New opt-in `Resolver` builder declaration mapping one directory class to one environment variable name plus its shape — exact directory (e.g. `State` ← `LITRPG_STATE_DIRECTORY`) or base for layout composition (e.g. a `--prefix`-style root). Undeclared classes behave exactly as today.
- Precedence becomes caller-supplied explicit root → declared environment value → discovery/development root → platform-standard root. A `PartialRoots::Some` value still beats the environment for its kind.
- Two new provenance values identifying environment-supplied roots, distinct from `ExplicitDirectory` (function argument) and `SysvarUserDirectory` (platform-provider input): `AppvarDirectory` for exact directories, `AppvarBase` for layout bases. The vocabulary stays closed at seven.
- Declared-variable reads: unset reads as absent (falls through); a zero-length value is malformed and returns the explicit tier's `EmptyPath` refusal, catching accidental `VAR=` exports loudly. Non-empty values validate exactly like the explicit tier (absolute and non-empty, Windows ambient-dependent shapes refused, relative values joined against `start`, typed error when relative with no start).
- No new dependencies. Additive in behavior: without declarations, resolved paths are byte-identical (labels follow the renamed vocabulary); downstream exhaustive matches must handle the two new variants, which is intended, compiler-surfaced, and pre-1.0.

## Capabilities

### New Capabilities

- `env-overrides`: opt-in per-class environment variable declarations, library-side read with validate-all-first semantics, precedence placement, and environment provenance reporting.

### Modified Capabilities

- `platform-roots`: Closed provenance vocabulary grows from five to seven values (implementations may report the new values, still may not invent others); Documented precedence chain gains the environment tier between the explicit root and discovery/development.
- `family-resolution`: `None` gap-filling runs the normal chain, which now includes the environment tier; `Some` values keep beating the environment for their kind.

Sequencing: this change is written against the renamed vocabulary and requires `rename-provenance` to land first.

## Impact

- `src/lib.rs`: builder declaration storage, per-resolve environment capture, precedence wiring in `resolve` and `resolve_partially_explicit`.
- `src/types.rs`: two new `Provenance` variants (`AppvarDirectory`, `AppvarBase`) plus two new declaration-error variants (`InvalidEnvDeclaration`, `DuplicateEnvDeclaration`); all matches over provenance must handle the new values (compiler-enforced).
- `tests/`: new contract tests with process-isolated environment control (child-process pattern, never mutating the test runner's own environment); existing suites gain no new behavior.
- Consumers: LitRPG can delete its caller-side CLI-vs-env chain for declared classes; Agentmux and Cistella are unaffected unless they declare.
