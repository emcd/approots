## MODIFIED Requirements

### Requirement: Closed provenance vocabulary

The library SHALL report exactly one provenance value for every resolved root, drawn from this closed set: `ExplicitBase`, `ExplicitDirectory`, `Discovery`, `SysvarUserDirectory`, `DefaultUserDirectory`, `AppvarBase`, `AppvarDirectory`. The set is closed: implementations SHALL NOT rename, add, or split values beyond these seven. Values name the root's source and shape: `Explicit*` for caller-passed roots, `Discovery` for layout derived from a discovered base, `Appvar*` for caller-declared environment variables (a base for layout composition vs an exact directory, paralleling the `Explicit*` pair), and user directories split into environment-influenced (`Sysvar*`) vs computed default (`Default*`). No value SHALL carry VCS-specific or topology-specific meaning in the generic framework.

#### Scenario: Every root carries a vocabulary value

- **WHEN** a caller resolves any root through any tier
- **THEN** the result carries exactly one of the seven enumerated provenance values.

#### Scenario: Argument and environment stay distinct

- **WHEN** a caller passes an explicit root while the declared variable for that class is also set
- **THEN** the explicit result carries `ExplicitDirectory` and an environment-sourced exact directory carries `AppvarDirectory`, never the same value for different sources.

### Requirement: Documented precedence

The library SHALL apply the precedence `caller-supplied explicit root → declared environment value → eligible development/supplied-directory root → platform-standard root` and report which tier supplied each result. Without any declaration the chain is identical to the three-tier order.

#### Scenario: Explicit root wins with caller provenance

- **WHEN** a caller provides an explicit root and development mode would also be eligible
- **THEN** the library returns the explicit root with `ExplicitDirectory` provenance.

#### Scenario: Declared environment beats repository discovery

- **WHEN** a declared variable holds a valid directory while a repository checkout is also discoverable
- **THEN** the library returns the environment value with `AppvarDirectory` (or `AppvarBase`-composed) provenance and never consults the checkout.

#### Scenario: Environment override beats repository discovery

- **WHEN** a caller passes an absolute override resolved from the environment (for example a per-test temporary state directory) while a repository checkout is also discoverable
- **THEN** the library returns the override and never consults the checkout.
