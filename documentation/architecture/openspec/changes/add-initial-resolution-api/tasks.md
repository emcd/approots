## 1. Core Types

- [ ] 1.1 Define `GitCheckout`, `DevelopmentScope`, provenance, and per-kind resolver entry points
- [ ] 1.2 Implement supplied-directory resolution with no-ancestor-search guarantee

## 2. Git Discovery

- [ ] 2.1 Implement subprocess discovery with explicit working directory and common-dir resolution
- [ ] 2.2 Implement prefix-based Git environment scrubbing and explicit failure outcomes
- [ ] 2.3 Implement optional Cargo package identity validation

## 3. Roots

- [ ] 3.1 Implement `.auxiliary` common/worktree candidates with caller-overridable layout (read-only)
- [ ] 3.2 Wrap platform-directory primitive with provenance preservation
- [ ] 3.3 Implement precedence chain with per-result provenance reporting

## 4. Verification

- [ ] 4.1 Cover clones, linked worktrees, clones-from-clones, nested invocations (normal and linked), and foreign checkouts, exercising actual Git command working-directory and environment construction
- [ ] 4.2 Cover contaminated Git environments, missing Git, symlinks, bare repos, source exports, cross-platform path shapes (Windows prefixes, drive-relative inputs), and sanitization collisions
- [ ] 4.3 Run `openspec validate --all --strict` and the crate test suite
