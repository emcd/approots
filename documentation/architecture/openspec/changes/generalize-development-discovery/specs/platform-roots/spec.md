## MODIFIED Requirements

### Requirement: State root resolves on Windows and macOS

On Windows, the library SHALL base the state root on the platform's local application-data base (`{FOLDERID_LocalAppData}`). On macOS, the library SHALL base the state root on the platform's application-support base (`~/Library/Application Support`). In the standard layout — the free `platform_root` function, or a `Resolver` whose `distinct_directory_classes` option is unset — the validated application name is appended literally, as for every other class, so every `DirectoryClass` resolves on every supported platform.

#### Scenario: State root on Windows

- **WHEN** a caller resolves the state class on Windows through the platform tier in the standard layout
- **THEN** the library returns an absolute path beneath the local application-data base, ending in the validated application name, with `PlatformDefault` provenance.

#### Scenario: State root on macOS

- **WHEN** a caller resolves the state class on macOS through the platform tier in the standard layout
- **THEN** the library returns an absolute path beneath `~/Library/Application Support`, ending in the validated application name, with `PlatformDefault` provenance.

#### Scenario: Platform classes may share one physical root

- **WHEN** a caller resolves `State` and `Cache` on Windows, or `State`, `Configuration`, and `Data` on macOS, in the standard layout
- **THEN** the aliased classes return the same physical root (base + application name), and the library makes no promise of per-kind physical isolation on the platform tier; consumers needing isolation supply distinct explicit roots or request distinct directory classes.

#### Scenario: Distinct directory classes on request

- **WHEN** a caller sets the resolver's `distinct_directory_classes` builder option and a resolution reaches the platform tier on Windows or macOS
- **THEN** the library appends the class's segment beneath the application directory (`<base>/<application>/<class>`), so every class addresses a distinct directory. Explicit-root and development-tier results are unchanged by the option, provenance is unchanged, and the free `platform_root` function always uses the standard layout. On Linux the option has no effect — leaving Linux unchanged is the chosen policy (distinct XDG overrides could otherwise produce aliased bases there too) — and the standard layout applies everywhere when the option is unset.

#### Scenario: Every class resolves on every supported platform

- **WHEN** a caller resolves each of the four classes on Linux, Windows, or macOS with a resolvable home/provider configuration, in the standard layout
- **THEN** each class returns an absolute path ending in the validated application name (typed unavailable remains possible only when the provider itself cannot be constructed, for example when no home directory exists).
