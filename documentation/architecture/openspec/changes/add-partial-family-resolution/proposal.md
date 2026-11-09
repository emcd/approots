## Why

Consumers need all four directory classes, not one: every application resolves configuration, state, cache, and data together, usually with some explicitly supplied (today's `--*-directory` flags carry complete per-kind directories such as `~/.config/agentmux-rnd`) and the rest left to discovery or platform defaults. Today that takes one `resolve` call per class with the caller repeating the precedence branching, and there is no single call that validates a caller-packaged family while filling its gaps.

Framing end-user cases driving this (and the crate generally):

- No path arguments: the program resolves standard platform
  directories with the application name in the proper components.
- Explicit full paths: the program uses them directly; the crate's
  role is validation and provenance, not resolution.
- Explicit root path: the program mates the local structure beneath
  the supplied root (e.g. a project checkout or a service base).
- Development mode: the program discovers a development tree root
  and mates the local structure beneath the discovered base.

## What Changes

- Add family input/output structs: `PartialRoots` (per-kind `Option<PathBuf>`) in, fully-resolved per-kind `ResolvedRoot` bundle out.
- Add `Resolver::resolve_partially_explicit`: per-kind `Some` values are validated exactly like the explicit tier (absolute, non-empty, no ambient Windows shapes; relative joins against `start`) and reported `CallerProvided`; per-kind `None` values run the normal chain (discovery from `start` when registered, else platform fallback — never ambient-CWD discovery).
- Validate every `Some` field before any gap-filling runs, mirroring the existing validate-before-discovery ordering.
- All-filled input degrades to a pure validator returning the bundle; nothing executes, nothing resolves.

## Capabilities

### New Capabilities

- `family-resolution`: family input/output structs and the partial-explicit resolution method with per-kind complete-or-missing semantics.

### Modified Capabilities

(none — per-kind explicit validation, discovery, platform, and precedence requirements are reused unchanged)

## Impact

- Additive public API only (`PartialRoots`, family output struct, one `Resolver` method); no existing API changes, no dependency changes.
- Intended first consumer: LitRPG/Agentmux migration, which can replace per-kind call loops with a single family call.
- 0.2.0 release vehicle (new API surface under zero-ver semver).
