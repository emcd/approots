## MODIFIED Requirements

### Requirement: Closed provenance vocabulary

The library SHALL report exactly one provenance value for every resolved root, drawn from this closed set: `ExplicitBase`, `ExplicitDirectory`, `Discovery`, `SysvarUserDirectory`, `DefaultUserDirectory`. The set is closed: implementations SHALL NOT rename, add, or split values. Values name the root's source and shape: `Explicit*` for caller-passed roots (a base for layout composition vs an exact directory), `Discovery` for layout derived from a discovered base (anchor selection stays discoverer-internal), and user directories split into environment-influenced (`Sysvar*`) vs computed default (`Default*`). No value SHALL carry VCS-specific or topology-specific meaning in the generic framework.

#### Scenario: Every root carries a vocabulary value

- **WHEN** a caller resolves any root through any tier
- **THEN** the result carries exactly one of the five enumerated provenance values.

### Requirement: Public name sanitization

The library SHALL expose a standalone, explicitly lossy name-sanitization helper keeping alphanumeric characters plus `-` and `_` and replacing everything else with `-`, so consumers stop duplicating the rule. Resolution accepts validated names literally and SHALL NOT promise cross-call, cross-application, or physical namespace uniqueness: distinct raw names may sanitize to the same value, filesystem case-folding and normalization vary, and a one-name read-only resolution has no candidate set to detect collisions against.

#### Scenario: Shared rule across consumers

- **WHEN** two consumers sanitize the same raw application name
- **THEN** both produce identical output from the shared helper.

#### Scenario: No uniqueness promised across calls

- **WHEN** two separate calls resolve names that sanitize to the same value
- **THEN** each call resolves its own name literally and neither claims the resulting namespaces are distinct.

### Requirement: Documented precedence

The library SHALL apply the precedence `caller-supplied explicit root → eligible development/supplied-directory root → platform-standard root` and report which tier supplied each result.

#### Scenario: Explicit root wins with caller provenance

- **WHEN** a caller provides an explicit root and development mode would also be eligible
- **THEN** the library returns the explicit root with `ExplicitDirectory` provenance.

#### Scenario: Environment override beats repository discovery

- **WHEN** a caller passes an absolute override resolved from the environment (for example a per-test temporary state directory) while a repository checkout is also discoverable
- **THEN** the library returns the override and never consults the checkout.

### Requirement: Platform-standard roots with preserved provenance

The library SHALL resolve platform-standard configuration, state, cache, and data roots through an established lower-level crate, preserving its distinction between environment-overridden and computed-default provenance where available.

#### Scenario: Platform root carries provenance

- **WHEN** a caller resolves the cache kind with no explicit or development root eligible
- **THEN** the library returns the platform cache root with `SysvarUserDirectory` or `DefaultUserDirectory` provenance as reported by the underlying primitive.

### Requirement: State root resolves on Windows and macOS

On Windows, the library SHALL base the state root on the platform's local application-data base (`{FOLDERID_LocalAppData}`). On macOS, the library SHALL base the state root on the platform's application-support base (`~/Library/Application Support`). In the standard layout — the free `platform_root` function, or a `Resolver` whose `distinct_directory_classes` option is unset — the validated application name is appended literally, as for every other class, so every `DirectoryClass` resolves on every supported platform.

#### Scenario: State root on Windows

- **WHEN** a caller resolves the state class on Windows through the platform tier in the standard layout
- **THEN** the library returns an absolute path beneath the local application-data base, ending in the validated application name, with `DefaultUserDirectory` provenance.

#### Scenario: State root on macOS

- **WHEN** a caller resolves the state class on macOS through the platform tier in the standard layout
- **THEN** the library returns an absolute path beneath `~/Library/Application Support`, ending in the validated application name, with `DefaultUserDirectory` provenance.

#### Scenario: Platform classes may share one physical root

- **WHEN** a caller resolves `State` and `Cache` on Windows, or `State`, `Configuration`, and `Data` on macOS, in the standard layout
- **THEN** the aliased classes return the same physical root (base + application name), and the library makes no promise of per-kind physical isolation on the platform tier; consumers needing isolation supply distinct explicit roots or request distinct directory classes.

#### Scenario: Distinct directory classes on request

- **WHEN** a caller sets the resolver's `distinct_directory_classes` builder option and a resolution reaches the platform tier on Windows or macOS
- **THEN** the library appends the class's segment beneath the application directory (`<base>/<application>/<class>`), so every class addresses a distinct directory. Explicit-root and development-tier results are unchanged by the option, provenance is unchanged, and the free `platform_root` function always uses the standard layout. On Linux the option has no effect — leaving Linux unchanged is the chosen policy (distinct XDG overrides could otherwise produce aliased bases there too) — and the standard layout applies everywhere when the option is unset.

#### Scenario: Every class resolves on every supported platform

- **WHEN** a caller resolves each of the four classes on Linux, Windows, or macOS with a resolvable home/provider configuration, in the standard layout
- **THEN** each class returns an absolute path ending in the validated application name (typed unavailable remains possible only when the provider itself cannot be constructed, for example when no home directory exists).

### Requirement: Honest provider adapter

The library SHALL obtain each platform root and its provenance from the same captured inputs, SHALL NOT infer provenance by path equality, and SHALL NOT label an opaque provider result `DefaultUserDirectory` when an environment variable may have overridden the provider's default. (An environment variable that merely inputs to computing a default, as `HOME`/`APPDATA` do on macOS/Windows, is not an override.) When the provider offers no root for a kind on a supported platform and this contract defines no mapping for that kind and platform, the library SHALL return a typed unavailable outcome or a documented fallback, never a fabricated path; missing-home behavior follows the same rule.

#### Scenario: Missing state root is typed, not fabricated

- **WHEN** the platform provider offers no state directory and no contract mapping covers that platform
- **THEN** the library returns the typed unavailable outcome (or the documented fallback) and no invented path.
