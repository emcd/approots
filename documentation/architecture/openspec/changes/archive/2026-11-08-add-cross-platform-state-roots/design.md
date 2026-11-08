## Context

The `directories` provider returns no state root on Windows or macOS (`state_dir` is `None` on both), so `State` is the one kind that does not resolve there today; the contract blesses that as a typed `Unavailable`. The provider's other bases are complete: on Windows it is built from the Known Folder API (`FOLDERID_RoamingAppData`, `FOLDERID_LocalAppData`, profile), and on macOS from the home Library layout (`Application Support`, `Caches`, `Preferences`). The operator wants Windows and macOS to be first-class — comparable to Python's `platformdirs`, which defines every kind on every platform — before the first release.

## Goals / Non-Goals

**Goals:**

- Every `DirectoryKind` resolves on Linux, Windows, and macOS with a stated mapping.
- Windows-only behavior (root-shape refusal, junction/alias discovery, non-ASCII names, all-kind availability) is executed in tests on Windows, not merely compiled.
- Keep the honest-adapter contract: provenance from captured inputs, no fabricated paths.

**Non-Goals:**

- Windows wide-character decoding of Git output for non-UTF8 checkout names (distinct follow-up requiring its own design).
- New dependencies, provider replacement, or any filesystem mutation.
- Changing Linux behavior or the provenance vocabulary.

## Decisions

- **State mapping:** Windows `State` bases on the provider's local application-data base (`FOLDERID_LocalAppData`); macOS `State` bases on its application-support base (`~/Library/Application Support`). These are the bases `platformdirs` uses for state on those platforms. The alternative — keeping `Unavailable` — was rejected by the operator in favor of platform parity.
- **Build on `directories`, no new crate:** the missing piece is one base selection in our adapter, not a provider gap in the other kinds. Rolling our own Known Folder calls would duplicate the provider for no gain.
- **Provenance stays `PlatformDefault`:** the Known Folder API and the macOS home layout are default-computing inputs; the honest-adapter requirement already distinguishes inputs-to-a-default from overrides. Nothing is relabelled.
- **Junction coverage by capability probe:** Windows test fixtures create a directory symlink where permitted and a junction otherwise, so alias discovery is exercised on either runner configuration without a new dependency.
- **Distinct directory classes as a builder option:** consumers needing kind isolation on aliased platforms set `distinct_directory_classes` once on the `Resolver` builder — deliberately per-application, never per call, so one application cannot split its own files across two layouts. The platform tier then appends the class segment beneath the application directory on Windows and macOS. Linux is left unchanged as a matter of policy — its default bases separate classes, though deliberately aliased XDG overrides could still produce shared roots there — and the free `platform_root` function always uses the standard layout.

## Risks / Trade-offs

- [Consumers may hold paths under a previously-`Unavailable` kind on Windows/macOS] → This is the intended behavior change; it lands before any release, so no published consumer can depend on the old outcome.
- [Platform kinds may alias to one physical root] → Accepted and documented rather than engineered away: platform roots join only base + application, so on Windows `State` and `Cache` are the same root (`{FOLDERID_LocalAppData}/<app>`), and on macOS `State`, `Configuration`, and `Data` are the same root (`~/Library/Application Support/<app>`; the macOS Configuration/Data alias predates this change). The provider and `platformdirs` layouts share bases the same way. Consumers SHALL NOT treat platform roots as per-kind isolated stores — in particular, cache cleanup on Windows would remove state held at the aliased root. A consumer needing kind isolation supplies distinct explicit roots or enables `distinct_directory_classes`. Changing the *default* layout to carry kind suffixes was considered and rejected, since it would deviate from the platform layout every other tool uses; the builder option is the intentional, opt-in deviation from that default.

## Open Questions

- Windows wide-character (non-UTF8) Git output decoding remains open for a later change.
