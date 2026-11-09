## 1. Types

- [x] 1.1 Add `PartialRoots` input struct (per-kind `Option<PathBuf>`) with constructor and validation helpers.
- [x] 1.2 Add family output struct (per-kind `ResolvedRoot`) and finalize naming in review.

## 2. Method

- [x] 2.1 Implement `Resolver::resolve_partially_explicit` with validate-all-Somes-first ordering and per-kind gap-filling through the existing chain.
- [x] 2.2 Wire relative-`Some` join-against-`start` and relative-without-start typed errors, mirroring the explicit tier.

## 3. Contract tests

- [x] 3.1 Pin all-filled validation (no execution, bundle returned) and per-kind `CallerProvided` provenance.
- [x] 3.2 Pin `None` gap-filling through discovery and platform, including `None`-with-no-start platform fallback (never CWD).
- [x] 3.3 Pin ordering: an invalid `Some` fails before any gap-filling executes (mutant-verified control).
- [x] 3.4 Pin base-vs-complete distinction: a base-shaped `Some` is used literally, never appended to.

## 4. Review and release

- [x] 4.1 Send proposal through Tier-1 review, then Tier-2 on approval.
- [x] 4.2 Implement, verify gates (nextest, clippy, openspec, cross-target checks), and re-verify through both tiers.
- [ ] 4.3 Release in 0.2.0.
