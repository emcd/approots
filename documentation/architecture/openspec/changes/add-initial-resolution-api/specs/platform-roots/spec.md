## ADDED Requirements

### Requirement: Platform-standard roots with preserved provenance

The library SHALL resolve platform-standard configuration, state, cache, and data roots through an established lower-level crate, preserving its distinction between environment-overridden and computed-default provenance where available.

#### Scenario: Platform root carries provenance

- **WHEN** a caller resolves the cache kind with no explicit or development root eligible
- **THEN** the library returns the platform cache root with `PlatformEnvironment` or `PlatformDefault` provenance as reported by the underlying primitive.

### Requirement: Propagation-safe normalization

The library SHALL normalize propagated roots to absolute paths lexically without canonicalizing, SHALL read blank environment values as absent, and SHALL reject explicit empty values.

#### Scenario: Stamped root survives child working directories

- **WHEN** a resolved root is propagated to a child with its own working directory
- **THEN** the root still addresses the same location because it was absolutized lexically and never rewritten through symlinks.

### Requirement: Public name sanitization

The library SHALL expose a standalone, explicitly lossy name-sanitization helper keeping alphanumeric characters plus `-` and `_` and replacing everything else with `-`, so consumers stop duplicating the rule. Resolution accepts validated names literally and SHALL NOT promise cross-call, cross-application, or physical namespace uniqueness: distinct raw names may sanitize to the same value, filesystem case-folding and normalization vary, and a one-name read-only resolution has no candidate set to detect collisions against.

#### Scenario: Shared rule across consumers

- **WHEN** two consumers sanitize the same worktree name
- **THEN** both produce identical output from the shared helper.

#### Scenario: No uniqueness promised across calls

- **WHEN** two separate calls resolve names that sanitize to the same value
- **THEN** each call resolves its own name literally and neither claims the resulting namespaces are distinct.

### Requirement: Closed layer lists with fault-on-unreadable

The library SHALL treat an explicitly supplied layer list as closed (it replaces the tier stack and never falls through to unsupplied roots) and SHALL report an unreadable layer as a fault rather than silently falling through to lower-precedence layers. A nonexistent explicit layer is retained as a candidate (absence is not a fault); "unreadable" means the root-level probe of that layer fails with an error other than not-found.

#### Scenario: Explicit list replaces defaults

- **WHEN** a caller supplies an explicit layer list that omits the platform default
- **THEN** resolution consults only the supplied layers and never the platform default.

#### Scenario: Unreadable layer does not fall through

- **WHEN** a supplied layer exists but cannot be read
- **THEN** the library returns a fault naming that layer instead of resolving from a lower-precedence layer.

### Requirement: Closed provenance vocabulary

The library SHALL report exactly one provenance value for every resolved root, drawn from this closed set: `CallerProvided`, `SuppliedDirectory`, `DevelopmentCommon`, `DevelopmentWorktree`, `PlatformEnvironment`, `PlatformDefault`. The set is closed: implementations SHALL NOT rename, add, or split values.

#### Scenario: Every root carries a vocabulary value

- **WHEN** a caller resolves any root through any tier
- **THEN** the result carries exactly one of the six enumerated provenance values.

### Requirement: Honest provider adapter

The library SHALL obtain each platform root and its provenance from the same captured inputs, SHALL NOT infer provenance by path equality, and SHALL NOT label an opaque provider result `PlatformDefault` when an environment variable may have overridden the provider's default. (An environment variable that merely inputs to computing a default, as `HOME`/`APPDATA` do on macOS/Windows, is not an override.) When the provider offers no root for a kind on a supported platform (for example no state directory on some macOS/Windows configurations), the library SHALL return a typed unavailable outcome or a documented fallback, never a fabricated path; missing-home behavior follows the same rule.

#### Scenario: Missing state root is typed, not fabricated

- **WHEN** the platform provider offers no state directory
- **THEN** the library returns the typed unavailable outcome (or the documented fallback) and no invented path.

### Requirement: Documented precedence

The library SHALL apply the precedence `caller-supplied explicit root → eligible development/supplied-directory root → platform-standard root` and report which tier supplied each result.

#### Scenario: Explicit root wins with caller provenance

- **WHEN** a caller provides an explicit root and development mode would also be eligible
- **THEN** the library returns the explicit root with `CallerProvided` provenance.

#### Scenario: Environment override beats repository discovery

- **WHEN** a caller passes an absolute override resolved from the environment (for example a per-test temporary state directory) while a repository checkout is also discoverable
- **THEN** the library returns the override and never consults the checkout.
