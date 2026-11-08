## 1. Renames and local structure

- [x] 1.1 Rename `DirectoryKind` to `DirectoryClass` across the API, tests, and in-flight spec references.
- [x] 1.2 Replace `DevelopmentLayout` with `LocalStructure` and mate it to arbitrary bases via `local_root`, preserving segment validation and provenance rules.
- [x] 1.3 Rename the builder's layout setter to match `LocalStructure`.

## 2. Discoverer model

- [x] 2.1 Add the discoverer trait, `DiscovererGenus` slots with one-per-genus registration refusal, and per-genus string arguments with typed unknown-key/value errors.
- [x] 2.2 Implement `VcsDiscovererGit` over the existing hardened discovery, including its typed `scope` option (`Common`/`Local`), and drop the direct `discover_checkout` entry point.
- [x] 2.3 Implement `DevfileDiscovererCargo` as both discoverer (upward marker search without a VCS base) and validator (marker presence and `[package]` identity against the discovery-context application name at a VCS base, with an optional expected-name argument overriding the application name where the two legitimately differ).
- [x] 2.4 Remove `DevelopmentScope` and the `development_mode` switch from the public interface; registration is the opt-in.

## 3. Verification and adoption

- [x] 3.1 Update the full test suite for the renamed and generalized interface, keeping every previously pinned behavior (topology, scrubbing, precedence, read-only) covered.
- [x] 3.2 Refresh the Agentmux and LitRPG migration briefs for the new names and the discoverer model.
- [ ] 3.3 Run nextest, clippy with warnings denied, strict OpenSpec validation, and cross-target test builds; confirm the GitHub `qa` matrix is green.

## 4. Review-gap verification (added at Tier-1)

- [x] 4.1 Pin every discovery state-machine scenario in tests: registered-genera matrix, Git failure versus absence routing, Cargo missing/unreadable/malformed/virtual outcomes at a selected base, standalone search bounds and no-marker absence, argument validation before any discovery work, and scope-omitted defaulting to Local.
- [x] 4.2 Pin exact `local_root` compositions and closed provenance values for supplied, Git Common/Local, Cargo-only, and platform-derived bases, including the never-double-append rule for application-namespaced roots.
