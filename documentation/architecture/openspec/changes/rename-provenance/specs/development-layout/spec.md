## MODIFIED Requirements

### Requirement: Exact local_root composition and provenance

`local_root` SHALL compose exactly `<base>/<prefix>/<class-segment>/<application>` — with the default structure, prefix `.auxiliary` and class segments `configuration`, `state`, `cache`, `data` — appending the application segment exactly once to the given base, and SHALL inherit the provenance of the `ResolvedRoot` supplied as its base rather than inventing one. Bases SHALL be absolute and non-empty, and on Windows SHALL NOT use ambient drive-relative or root-relative shapes: with no explicit-start parameter, unresolved bases are typed refusals — `EmptyPath` for empty, `RelativeStartDirectory` for relative, `InvalidName` for Windows ambient-dependent shapes — never propagated relative roots. Valid bases are propagated lexically and SHALL NOT be canonicalized. Rawness (not already application-namespaced) SHALL remain an unchecked caller precondition: `local_root` composes onto whatever valid base it is given, including a raw base that itself ends in the application name. A `platform_root` result is a complete root, not a `local_root` base, and `Resolver` SHALL never feed its completed platform-tier result into `local_root`. In `Resolver`, only the discovered and supplied tiers apply the local structure; the platform tier SHALL NOT have the structure applied to it (preserving the platform contract is a non-goal constraint on this change), and `distinct_directory_classes` SHALL affect only platform-tier results, never `local_root` compositions. The application namespace guarantees (validated literal name, no cross-call uniqueness promise) SHALL apply unchanged.

#### Scenario: Supplied base example

- **WHEN** a caller mates the default structure onto an absolute supplied base (POSIX example `/srv/inst1`; any absolute base on the host platform) for the state class of application `litrpg`
- **THEN** the result is that base joined with `.auxiliary/state/litrpg` (`/srv/inst1/.auxiliary/state/litrpg` on POSIX) with `ExplicitBase` provenance.

#### Scenario: Discovered examples

- **WHEN** the Git discoverer selects the common root `/repo` (`scope` = `Common`) for configuration, or the worktree root `/repo-wt` (`scope` = `Local`) for state (POSIX examples; any absolute discovered base on the host platform)
- **THEN** the results are those bases joined with `.auxiliary/configuration/<application>` and `.auxiliary/state/<application>` respectively, each with `Discovery` provenance; a Cargo-only discovered base likewise carries `Discovery` provenance. Anchor selection is the discoverer's `scope` option, and no VCS topology leaks into the provenance.

#### Scenario: Raw base ending in the application name still composes

- **WHEN** a caller mates the default structure onto an absolute raw base ending in the application name (POSIX example `/repo/litrpg`) for the state class of application `litrpg`
- **THEN** the result is that base joined with `.auxiliary/state/litrpg` (`/repo/litrpg/.auxiliary/state/litrpg` on POSIX): rawness is a caller precondition the library does not check, and composition appends the application segment exactly once relative to the given base.

#### Scenario: Unresolved bases are refused

- **WHEN** a caller mates the structure onto an empty, relative, or Windows ambient-dependent base
- **THEN** the library returns `EmptyPath`, `RelativeStartDirectory`, or `InvalidName` respectively instead of a propagated relative root.

#### Scenario: Platform root is not a local_root base

- **WHEN** a caller holds a `platform_root` result (already ending in the application name)
- **THEN** that root is used as a complete root; `Resolver` never feeds its completed platform-tier result into `local_root`, and platform-tier results keep their exact platform paths and `SysvarUserDirectory`/`DefaultUserDirectory` provenance.
