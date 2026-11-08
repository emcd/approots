## Why

The development side of the API is Git-centric in ways the operator's original brief was not. `DevelopmentScope` bakes Git's common/worktree topology into the resolver's vocabulary, although not every VCS has a worktree concept; `DevelopmentLayout` describes a local directory structure but mates it only to Git-discovered bases, although the motivating consumers (Agentmux's and LitRPG's `-rnd` deployments) run against manually supplied configuration and state bases outside any development tree; and `DirectoryKind`/`layout` naming is inconsistent with the `distinct_directory_classes` vocabulary adopted in the cross-platform change. This generalization lands before the 0.1 release, while renames are still free.

## What Changes

- **BREAKING** Rename `DirectoryKind` to `DirectoryClass` throughout the API, aligning with `distinct_directory_classes`.
- **BREAKING** Replace `DevelopmentLayout` with `LocalStructure`: the same validated segment scheme, mated to any base — a discovered root, a platform root, or a caller-supplied base — via a `local_root` resolution that reports the base's provenance. This lets parallel servers run against distinct supplied bases (for example per-instance `-rnd` state bases) with one shared structure instead of contending over the platform-provided set.
- **BREAKING** Remove `DevelopmentScope` from the public interface. Anchor selection becomes a discoverer option: the Git discoverer deserializes a `scope` argument into its own `Scope { Common, Local }` enum.
- **BREAKING** Replace the Git-specific discovery wiring in `Resolver` with a discoverer model: a discoverer trait, genus slots (`DiscovererGenus::{Vcs, Devfile}`, at most one discoverer per genus per resolver), and string-keyed per-genus arguments (`.discoverer_argument(DiscovererGenus::Vcs, "scope", "common")`) that each discoverer deserializes into its own typed options with typed errors for unknown keys or values. First implementations: `VcsDiscovererGit` (wrapping the existing hardened Git discovery as its private machinery; the direct `discover_checkout` entry point is dropped, since registering the discoverer is the intent signal) and `DevfileDiscovererCargo` (marker discovery and identity validation). `VcsDiscovererHg` and `DevfileDiscovererPyproject` are named as designed-for slots, not implemented here.
- **BREAKING** Remove the `development_mode` switch: registering a discoverer is the explicit opt-in, so discovery stays caller-enabled and never becomes implicit detection.
- Preserve all existing guarantees: read-only resolution, closed provenance vocabulary, lexical propagation with comparison-only canonicalization, precedence (explicit → discovered/supplied → platform), and hardened subprocess discovery.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `development-layout`: layout and scope requirements are rewritten around `LocalStructure` mating to arbitrary bases and discoverer-supplied anchors.
- `git-discovery`: discovery requirements are reframed as the Git discoverer's contract behind the generalized discoverer interface.
- `supplied-resolution`: supplied bases gain structure mating via `local_root`.
- `platform-roots`: reference migration only — the state-root requirement's `DirectoryKind` reference becomes `DirectoryClass`; no platform behavior changes.

## Impact

- `src/lib.rs`, `src/types.rs`: renames, `DiscovererGenus`, discoverer argument storage, resolver rewiring.
- `src/development.rs`: becomes the local-structure module.
- `src/git.rs`: Git discovery behind `VcsDiscovererGit`; `discover_checkout` dropped, its implementation private to `VcsDiscovererGit`.
- New discoverer module for the trait and the Cargo devfile discoverer.
- Specs, tests, and the archived consumer mappings (Agentmux/LitRPG briefs reference the old names and must be refreshed on adoption).
- Out of scope: implementing Hg or Pyproject discoverers, out-of-process discoverers (recorded at `approots:ideas/1`), Windows wide-character Git decoding, and any release action.
