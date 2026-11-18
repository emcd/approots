# Design: provenance rename

## Context

The twice-approved six-value vocabulary mixes schemes (`CallerProvided` vs `SuppliedDirectory` vs `DevelopmentCommon`) and carries Git's worktree topology in generic-framework clothing. The `VcsDiscovererGit` split established the rule — VCS specifics live in the discoverer — but the vocabulary was never swept. The operator requires a linguistically consistent, source-first series, enforced pre-1.0 while renames are free. The environment change is sequenced after this rename and takes its names from the new set.

## Goals / Non-Goals

Goals: one consistent source × shape matrix; zero Git-isms (or any VCS-isms) in generic names; byte-identical resolution (labels only); site values explicitly deferred with names reserved.

Non-Goals: no precedence, validation, discovery, or path changes; no new directory classes (a temporary-directories class stays future work); no discoverer-metadata attachment mechanism beyond noting it as a future extension point.

## Decisions

1. **Source-first matrix: Explicit / Discovery / Appvar / User × Base / Directory.** Each value names where the root came from and whether it is an exact directory or a base for layout composition. Rationale: matches how consumers branch (exact-use vs compose-beneath) and how roots are supplied (argument, discovered, application variable, platform). Alternative (mechanism-only Explicit/Environment/Discovery without the shape split) was rejected: it merges exact-use and compose-beneath roots that the family and local-structure contracts treat differently.

2. **`DevelopmentCommon` + `DevelopmentWorktree` → single `Discovery`.** Rationale: the common/worktree anchor is one VCS's topology; anchor selection already lives discoverer-internally (the Git discoverer's `scope` option), so the generic vocabulary needs only "derived from a discovered base." A discoverer that wants to expose anchor metadata may do so in future through an extension, but this change adds no metadata channel — `Discovery` stays a plain value.

3. **Mandatory `Sysvar`/`Default` split for user directories.** Rationale: preserves the honest-adapter distinction Tier-2 extracted (environment-influenced vs computed default), which collapsing would undo. Same split reserved for future site values.

4. **No site values now.** Rationale: the resolver produces no site roots, so normative site values would be untestable contract surface — the exact class of finding Tier-2 raised on provenance before. Names (`SysvarSiteDirectory`, `DefaultSiteDirectory`) are reserved in the site-wide discovery todo, not the vocabulary.

5. **Reviewer gate on VCS terms.** Rationale: the miss at the `VcsDiscovererGit` split must not recur — the packet instructs both tiers to verify the generic vocabulary contains no topology-specific terms.

## Risks / Trade-offs

- [Consumer match breakage] Every exhaustive `match` downstream fails to compile. → Mitigation: intended and compiler-surfaced; pre-1.0 minor; consumers change labels only, paths are identical.
- [Scope-signal loss] `Common`/`Worktree` no longer visible in provenance. → Mitigation: anchor selection was already discoverer-internal; no consumer policy was shown to branch on the pair (LitRPG's regime converged them); both tiers confirm.
- [Churn if renamed again] → Mitigation: this is the series the operator asked for repeatedly; take the breakage once, pre-1.0.

## Migration Plan

Rename enum variants and update all matches, specs, scenarios, and the in-flight environment change (`AppvarBase`/`AppvarDirectory` layer onto this set). Consumers update matches label-for-label. No data, path, or precedence migration: resolution output is unchanged apart from labels. Rollback is revert.

## Open Questions

- None blocking. Exact scenario wording per capability is settled in the deltas; reviewers may bikeshed individual names.
