## Why

Windows and macOS are second-class in the current platform-roots implementation: the `directories` provider offers no state root on either platform, so `State` falls back to a typed `Unavailable` outcome there while the other three kinds resolve. A library whose purpose is platform-standard application directories — comparable in spirit to Python's `platformdirs` — should resolve all four kinds on every supported platform, with the mapping stated in the contract rather than left as a provider gap.

## What Changes

- Define the `State` mapping on Windows and macOS on top of the existing provider: Windows → its local application-data base (`{FOLDERID_LocalAppData}`), macOS → its application-support base (`~/Library/Application Support`). These are the bases `platformdirs` uses for state on those platforms.
- Execute the Windows-only root-shape refusal (drive-relative and root-relative explicit roots) in tests on Windows, replacing today's compile-only coverage.
- Add Windows/macOS platform-root availability coverage: every kind resolves to an absolute path ending in the literal validated application name, on every supported platform.
- Add Windows discovery coverage through a directory junction or symlink (whichever the runner permits) and a non-ASCII checkout name.
- Amend the platform-roots contract: the "no state root on some configurations" example is narrowed to genuinely unresolvable provider cases now that Windows and macOS have a defined mapping.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `platform-roots`: the platform availability requirement gains a defined `State` mapping for Windows and macOS, and the honest-adapter unavailable example is narrowed accordingly.

## Impact

- `src/platform.rs`: provider-default state base selection per platform.
- `tests/resolution.rs`, `tests/env_scrub.rs`: cross-platform availability and Windows-specific behavior tests.
- No API shape changes, no new dependencies, no provenance vocabulary changes. Linux behavior is unchanged.
- Out of scope: Windows wide-character decoding of Git output for non-UTF8 checkout names (a distinct follow-up), filesystem mutation of any kind, and provider replacement.
