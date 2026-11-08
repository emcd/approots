## 1. Renames and local structure

- [ ] 1.1 Rename `DirectoryKind` to `DirectoryClass` across the API, tests, and in-flight spec references.
- [ ] 1.2 Replace `DevelopmentLayout` with `LocalStructure` and mate it to arbitrary bases via `local_root`, preserving segment validation and provenance rules.
- [ ] 1.3 Rename the builder's layout setter to match `LocalStructure`.

## 2. Discoverer model

- [ ] 2.1 Add the discoverer trait, `DiscovererGenus` slots with one-per-genus registration refusal, and per-genus string arguments with typed unknown-key/value errors.
- [ ] 2.2 Implement `VcsDiscovererGit` over the existing hardened discovery, including its typed `scope` option (`Common`/`Local`), and drop the direct `discover_checkout` entry point.
- [ ] 2.3 Implement `DevfileDiscovererCargo` as both discoverer (upward marker search without a VCS base) and validator (marker presence and `[package]` identity against the discovery-context application name at a VCS base, with an optional expected-name argument overriding the application name where the two legitimately differ).
- [ ] 2.4 Remove `DevelopmentScope` and the `development_mode` switch from the public interface; registration is the opt-in.

## 3. Verification and adoption

- [ ] 3.1 Update the full test suite for the renamed and generalized interface, keeping every previously pinned behavior (topology, scrubbing, precedence, read-only) covered.
- [ ] 3.2 Refresh the Agentmux and LitRPG migration briefs for the new names and the discoverer model.
- [ ] 3.3 Run nextest, clippy with warnings denied, strict OpenSpec validation, and cross-target test builds; confirm the GitHub `qa` matrix is green.
