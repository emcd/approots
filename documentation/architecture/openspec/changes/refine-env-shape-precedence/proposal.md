## Why

The env-overrides contract declares the two environment shapes unordered, which is unimplementable: a caller that declares both an exact directory and a layout base for one class (for example `MYAPP_CONFIGURATION_DIRECTORY` alongside `MYAPP_PREFIX`) needs a deterministic winner, and the only sane one is the more specific value. Operator direction pins it: the exact directory wins.

## What Changes

- A class may declare one exact-directory variable AND one layout-base variable (previously any second declaration for a class was a `DuplicateEnvDeclaration` refusal). Declaring the same shape twice for one class remains a duplicate.
- Precedence within the environment tier becomes exact directory before layout base; the tier as a whole keeps its place between the explicit argument and discovery.
- Both declared values validate after capture and before any tier is chosen (validate-all-first extends within the class); an invalid value of either shape is a typed refusal even when the other shape would win. Explicit `Some` and explicit arguments still mask the class's entire declaration.
- No other behavior changes: capture-once with per-distinct-name caching, masking, and barrier semantics are untouched.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `env-overrides`: declaration rules (one declaration per shape per class), intra-tier precedence (exact directory beats layout base), and validation of both values.

## Impact

- `src/lib.rs`: declaration storage keyed by (class, shape); capture checks declarations, `apply_env` validates both values before choosing and prefers the exact directory.
- `tests/env_overrides.rs`: both-shapes coverage (exact wins, base fills when exact is unset, invalid either-shape value refuses).
- Consumers: undeclared behavior identical; single-shape declarations identical. Only callers who hit the old duplicate refusal gain new (working) behavior.
