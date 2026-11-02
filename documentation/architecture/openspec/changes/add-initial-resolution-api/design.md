## Context

Agentmux resolves layered configuration roots with explicit precedence and stamps bring-up context into spawned members. LitRPG resolves Git checkout topology; it migrated from linked worktrees to independent clones made from a local clone, so its shared-versus-per-worktree distinction is vestigial and everything is per-clone — the equal-roots case the topology model already covers. Both need the same hardened Git discovery and explicit common-versus-worktree scope, without the crate touching parsing, creation, or permissions.

Stakeholders: Agentmux and LitRPG owners (first consumers); NbMcp and Cistella owners (later input). Tier-1 review via `reviewer@rust-foundation`; Tier-2 via `advisor@infrastructure`.

## Goals / Non-Goals

**Goals:**

- One read-only resolution API covering supplied directories, Git discovery, `.auxiliary` layout, and platform roots.
- Explicit caller opt-in to development behavior; deterministic fallbacks.
- Support local directory trees (e.g. Agentmux R&D bundle roots such as `~/.config/agentmux-rnd`) via explicitly supplied, possibly relative bases resolved against the caller-supplied explicit start directory, never silently against ambient cwd.
- Provenance on every resolved root.

**Non-Goals:**

- Consumer application configuration parsing, merging, creation, permissions, locking, or writes. (Cargo.toml identity parsing is the sole declared exception.)
- Environment-variable nomenclature or CLI handling (caller passes explicit paths).
- Agentmux/LitRPG migration (sequence: provisional API, then migrations, then stabilization; migrations are follow-up changes, not part of this one).

## Decisions

- **One-kind-at-a-time resolution over an aggregate root set.** The proposal note flags the aggregate as possibly premature. Per-kind functions let each consumer resolve only what it needs and keep scope selection explicit. Alternative (eager `ApplicationRoots`) considered; deferred until both migrations prove the shape.
- **Explicit `GitCheckout` topology struct** (`worktree_root`, `common_worktree_root`, `git_common_dir`) instead of one ambiguous root. The Git-specific name keeps any future VCS under its own named type; `DevelopmentScope` stays generic. Makes linked-worktree sharing a caller decision via `DevelopmentScope::Common` vs `::Worktree`.
- **Layered roots instead of an `overrides/` convention.** Local customization is expressed as ordered supplied layers searched first-match-wins (the Agentmux model, `openspec/changes/archive/2026-07-28-layer-configuration-roots/`). The crate defines no `overrides/` subdirectory; LitRPG's `overrides/mcp.toml` maps onto one extra layer at migration time. Layer lists are CLOSED (an explicit list replaces the tier stack; defaults do not leak back in), empty elements are rejected rather than read as the working directory, and a path containing the env separator is expressed via the repeatable-flag form, which is the escape hatch. Union-by-identifier across layers (Agentmux bundle definitions) is consumer policy; the library resolves ordered directory roots, and per-file first-match lookup is the consumer's use of them. File-specific resolution and absence semantics (e.g. `mcp.toml`) stay with consumers. An unreadable layer is a reportable fault, never silent fallthrough (Agentmux `2026-08-18-distinguish-unreadable-configuration-layers`), and there is one state-root rule in every build profile with propagation to spawned children (`2026-07-29-unify-state-root-resolution`).
- **Git via subprocess matching `git rev-parse` semantics** (`--show-toplevel`, `--git-common-dir`), invoked with an explicit working directory and prefix-based environment scrubbing. Avoids a heavy dependency for v1; revisit only if semantics diverge.
- **Cargo package marker first; Python marker deferred.** Agentmux (more mature path behavior) integrates first and exercises the Cargo path. Generic marker framework explicitly avoided for v1.
- **Propagate lexically, compare canonically.** Stamped/propagated roots use lexical `std::path::absolute` and NEVER canonicalize: canonicalizing rewrites an operator-named path through symlinks, and a relative stamped root re-resolves against each child's own cwd. Comparison (e.g. cwd-vs-member matching) canonicalizes best-effort so symlink aliasing does not false-mismatch. Return candidates without creating them; canonicalize only for comparison, never for propagation. Empty handling differs per tier deliberately: env blank reads as absent, explicit empty is rejected.
- **Grammar-then-join-check with typed ambiguity.** Every joined segment satisfies a token grammar (no `/`, no empty, no `.`/`..`, plus domain separators) BEFORE joining, with traversal-only segments rejected at each join call site. Ambiguity (e.g. symlinked dirs collapsing under comparison) surfaces as a typed error, never first-match-wins.
- **Delegate platform conventions to an established crate.** The crate adds policy and provenance, not directory tables.
- **Application-name parameterization for deployment suffixes.** Agentmux operations carry per-deployment names (e.g. `agentmux-rnd`) via env overrides while the application directory is a compile-time const. The crate takes a caller-supplied application name that the XDG/default tiers derive `<base>/<name>` from; explicit flag and full-path env tiers bypass it unchanged.

## Risks / Trade-offs

- [Ambiguous shared-vs-local default recreates races] → No default scope; every development resolution takes an explicit `DevelopmentScope`.
- [Marker-less discovery adopts a foreign checkout] → Marker-less mode is an intentional caller choice; Git-aware discovery without a marker resolves only when the caller opts in, and release callers must not auto-adopt nearby checkouts.
- [Eager canonicalization fails on absent paths] → Mitigated by the no-canonicalize-outputs decision above.
- [API stabilized before both migrations] → Stabilize only after Agentmux then LitRPG migrations; keep v1 surface minimal.

## Migration Plan

None in this change (new API, no consumers). Follow-ups: Agentmux migration first, LitRPG second, then stabilization. Rollback: not applicable; unreleased API can be revised in place.

## Open Questions

- Cargo-only vs Cargo-plus-Python markers for v1 (leaning Cargo-only; Agentmux interview pending).
- Caller-provided provenance labels: DECIDED — provenance is the closed normative enum only; opaque caller labels are out of v1.
- Which lower-level platform-directory crate to wrap.
- Whether to cache Git discovery per resolver instance or not at all.
- Whether v1 should include a generalizable diagnostics-stream root (inscriptions-style: defaults beneath the state root, independently overridable; filenames stay consumer-specific) or defer it.
- Whether a same-FD anchored-open helper (O_NOFOLLOW open as a regular file returning the FD for fstat, bounded read, and post-read re-check, with absent/symlink/non-regular distinct) belongs in this crate or a separate one; resolution alone never stats-then-reopens. If included, it must stay TOCTOU-shut and keep host-sources-must-pre-exist as caller-enforced, never library-assumed.
- Scope discovery stays out: any enumeration helpers must be clearly separate from resolution so discovered paths are never mistaken for trusted roots.
- Whether v1 should include filename-composition helpers (parts plus one composed helper with the `{project}--{worktree}.log`, single-name-when-equal default) or directories only; any scope selector must keep per-worktree paths unmoved.
- Drop the scaffold smoke test at the next template re-population that carries zero-test tolerance; it asserts nothing and should not outlive its reason.
- Remote inspection for cross-clone identity (e.g. hashing the ultimate remote, Opencode-style): DEFERRED. It yields identity, not a usable directory — the ultimate remote may be a URL or a read-only/absent clone, and it cannot authorize mutation under an upstream clone's `.auxiliary/state`. Revisit only as a standalone identity helper, never as a root.
