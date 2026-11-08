## ADDED Requirements

### Requirement: State root resolves on Windows and macOS

On Windows, the library SHALL base the state root on the platform's local application-data base (`{FOLDERID_LocalAppData}`). On macOS, the library SHALL base the state root on the platform's application-support base (`~/Library/Application Support`). In the standard layout — the free `platform_root` function, or a `Resolver` whose `distinct_directory_classes` option is unset — the validated application name is appended literally, as for every other kind, so every `DirectoryKind` resolves on every supported platform.

#### Scenario: State root on Windows

- **WHEN** a caller resolves the state kind on Windows through the platform tier in the standard layout
- **THEN** the library returns an absolute path beneath the local application-data base, ending in the validated application name, with `PlatformDefault` provenance.

#### Scenario: State root on macOS

- **WHEN** a caller resolves the state kind on macOS through the platform tier in the standard layout
- **THEN** the library returns an absolute path beneath `~/Library/Application Support`, ending in the validated application name, with `PlatformDefault` provenance.

#### Scenario: Platform kinds may share one physical root

- **WHEN** a caller resolves `State` and `Cache` on Windows, or `State`, `Configuration`, and `Data` on macOS, in the standard layout
- **THEN** the aliased kinds return the same physical root (base + application name), and the library makes no promise of per-kind physical isolation on the platform tier; consumers needing isolation supply distinct explicit roots or request distinct directory classes.

#### Scenario: Distinct directory classes on request

- **WHEN** a caller sets the resolver's `distinct_directory_classes` builder option and a resolution reaches the platform tier on Windows or macOS
- **THEN** the library appends the kind's class segment beneath the application directory (`<base>/<application>/<kind>`), so every kind addresses a distinct directory. Explicit-root and development-tier results are unchanged by the option, provenance is unchanged, and the free `platform_root` function always uses the standard layout. On Linux the option has no effect — leaving Linux unchanged is the chosen policy (distinct XDG overrides could otherwise produce aliased bases there too) — and the standard layout applies everywhere when the option is unset.

#### Scenario: Every kind resolves on every supported platform

- **WHEN** a caller resolves each of the four kinds on Linux, Windows, or macOS with a resolvable home/provider configuration, in the standard layout
- **THEN** each kind returns an absolute path ending in the validated application name (typed unavailable remains possible only when the provider itself cannot be constructed, for example when no home directory exists).

## MODIFIED Requirements

### Requirement: Honest provider adapter

The library SHALL obtain each platform root and its provenance from the same captured inputs, SHALL NOT infer provenance by path equality, and SHALL NOT label an opaque provider result `PlatformDefault` when an environment variable may have overridden the provider's default. (An environment variable that merely inputs to computing a default, as `HOME`/`APPDATA` do on macOS/Windows, is not an override.) When the provider offers no root for a kind on a supported platform and this contract defines no mapping for that kind and platform, the library SHALL return a typed unavailable outcome or a documented fallback, never a fabricated path; missing-home behavior follows the same rule.

#### Scenario: Missing state root is typed, not fabricated

- **WHEN** the platform provider offers no state directory and no contract mapping covers that platform
- **THEN** the library returns the typed unavailable outcome (or the documented fallback) and no invented path.
