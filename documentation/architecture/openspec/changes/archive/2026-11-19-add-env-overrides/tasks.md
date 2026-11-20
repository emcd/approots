## 1. Declaration and precedence

- [x] 1.1 Add per-class environment declaration to the `Resolver` builder (one variable name plus exact/base shape per class, unset by default) with single-capture read at resolve start; empty/`=`-containing names and repeated per-class declarations are typed `InvalidEnvDeclaration`/`DuplicateEnvDeclaration` refusals raised at capture.
- [x] 1.2 Read values with `var_os` (native `OsString` preservation, never lossy); wire precedence explicit → environment → discovery/development → platform in `resolve`, keeping `Some` above environment in `resolve_partially_explicit`.
- [x] 1.3 Add the `AppvarDirectory`/`AppvarBase` provenance values and handle them at every provenance match (compiler-checked).

## 2. Contract tests

- [x] 2.1 Pin declared exact-directory and base supply with environment provenance preserved; explicit beats environment.
- [x] 2.2 Pin empty semantics: unset falls through, zero-length returns `EmptyPath`; whitespace-only validates as a relative value; invalid environment values (relative-without-start, Windows ambient shapes) return the matching explicit-tier refusals.
- [x] 2.3 Pin invalid declarations (`InvalidEnvDeclaration`, `DuplicateEnvDeclaration`) surfacing before any tier resolves, and non-UTF8 values surviving byte-identically.
- [x] 2.4 Pin undeclared-class path identity and per-class read isolation; explicit masking malformed values and declaration problems; shared variable read once and reused across classes (counter-pinned); family gap-filling from the environment tier with `Some`-beats-environment, all-filled masking environment entirely (even invalid values), valid roots never masking another kind's malformed environment, and invalid gap environment failing before any gap discovery (proven with a recording discoverer showing zero invocations).
- [x] 2.5 Pin top-level family snapshot (one capture for gap kinds after `Some` validation and the all-filled short-circuit), argument validation running when environment wins (bad-bag control), no-start absolute-environment supply, and malformed-relative-with-no-start erroring instead of falling back to platform.
- [x] 2.6 Run all environment tests in child processes with fixture-owned environments (never mutating the runner's process environment), including a poison control with hostile ambient variables.

## 3. Verification and review

- [x] 3.1 Verify gates (nextest, clippy with warnings denied, openspec strict, cross-target checks) and confirm the vocabulary/precedence deltas against the existing suites.
- [x] 3.2 Route the packet to Tier-1 (sequenced after rename-provenance approval), then Tier-2 on Tier-1 approval, with no pushes of unapproved commits. DONE: both tiers approved.
