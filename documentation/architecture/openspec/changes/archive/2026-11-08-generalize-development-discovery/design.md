## Context

The initial API grew around its first discoverer: `Resolver` knows Git exists, `development_root` consumes `GitCheckout` directly, and `DevelopmentScope::{Common, Worktree}` promotes one VCS's topology (a shared common directory plus linked worktrees) into the library's core vocabulary. The operator's brief pointed the other way — `GitCheckout` was named precisely so the generic layer would not be tied to Git — and the field evidence since (LitRPG's clones-of-clone regime, where common and worktree roots converge; two consumers running development instances against supplied `-rnd` bases outside any checkout) shows the development concepts need to separate into: bases (discovered or supplied), a local structure mated onto a base, and discoverers that produce bases under caller-supplied arguments.

## Goals / Non-Goals

**Goals:**

- A VCS-agnostic core: no Git topology (worktree, common directory) in the resolver's vocabulary.
- `LocalStructure` mating to any base with the base's provenance, covering the parallel-instances use case (distinct supplied bases, one structure).
- A discoverer model with category slots and typed per-discoverer options, implementing Git (VCS) and Cargo (devfile) first.
- Naming consistency: `DirectoryClass`, `VcsDiscovererGit` form for discoverer types, `DiscovererGenus` for the slot enum ("class" is reserved for directory classes).

**Non-Goals:**

- Implementing Mercurial or Pyproject discoverers (the trait and slots are designed for them; they are follow-ups).
- Out-of-process discoverers (recorded at `approots:ideas/1`; the argument-bag design must not preclude a JSON wire form later).
- Changing platform-tier behavior, provenance values, or the read-only guarantee.

## Decisions

- **`LocalStructure` replaces `DevelopmentLayout`.** Same validated segments (prefix plus per-class segments) and containment rules; the mating function takes any base path and the provenance that base carries. Development bases are one source among three (discovered, supplied, or a raw provider base — never a completed, application-namespaced `platform_root` result), not the structure's owner.
- **Discoverers sit behind a trait with genus slots.** `DiscovererGenus::{Vcs, Devfile}` is a closed enum; a resolver holds at most one discoverer per genus, so the genus unambiguously addresses argument routing and prevents option-key collisions between a VCS and a devfile discoverer. One-per-genus is a stated constraint: multi-VCS probing or multi-marker monorepos use a composite discoverer inside one slot, or separate resolvers.
- **Arguments are a string-keyed bag per category, deserialized inside the discoverer.** Callers pass `.discoverer_argument(category, key, value)`; each discoverer converts values into its own typed options (the Git discoverer's `scope` becomes `Scope::{Common, Local}`) and rejects unknown keys/values with a typed error naming the discoverer, key, and value. Stringified JSON was rejected (callers hand-assembling text, opaque parse failures); a `serde_json::Value` API was deferred (flat options today, keeps the public surface dependency-light), with the note that the future process protocol can translate the same bag to JSON mechanically.
- **No scope enum in the core interface.** `DevelopmentScope` is removed rather than renamed; anchor selection is the Git discoverer's `scope` option. A discoverer whose system has no shared/local distinction simply has no such option.
- **Registration replaces the development-mode switch.** A resolver with no discoverers never discovers; registering one is the explicit opt-in the no-implicit-detection rule requires. This removes a boolean whose only meaning was "a discoverer is configured."
- **One discovery yields one base.** A discovered base is a single directory concentrating all directory classes under the caller's `LocalStructure`. Anchor multiplicity (Git's worktree vs common root) is resolved inside the discoverer by its arguments before returning, so the generic shape carries no shared-root concept. A discoverer returning several simultaneous bases (workspace members, monorepo roots) would be enumeration, which stays outside the resolution contract.
- **Devfile discoverers both discover and validate.** Given a VCS base they validate that their marker files are present at that base and that the project identity inside matches; with no VCS discoverer registered they search upward from the start directory for their marker and propose the containing directory as the base. Identity validation receives the application name through the discovery context (an explicit expected-name argument may override it where project and application names legitimately differ) and checks the pertinent entry — `[package] name` in `Cargo.toml`, `[project] name` in `pyproject.toml`. A mismatch is a typed error, never a fallthrough.
- **No direct Git entry point.** `discover_checkout` is dropped rather than renamed: registering `VcsDiscovererGit` is itself the signal that Git discovery is wanted, and a second public path to the same machinery would invite callers to bypass the generic model. The hardened discovery implementation becomes the discoverer's private machinery, and consumers select the Git topology they want through the discoverer's `scope` argument.

## Risks / Trade-offs

- [Stringly-typed arguments lose compile-time checking at the call site] → Discoverers deserialize eagerly with typed errors, publish their option keys as constants, and tests pin each known key; the alternative (a struct per discoverer in the resolver API) would couple the core to every discoverer's options.
- [One-per-category frustrates a future multi-marker consumer] → Composite discoverers inside a slot are the documented escape hatch; relaxing to multiple per category later is additive, tightening would be breaking — so start with the constraint.
- [Renames churn the two consumer mappings already circulated] → Both consumers are pre-migration; refreshed briefs are a task in this change, and landing before the 0.1 release keeps the old names unpublished.
