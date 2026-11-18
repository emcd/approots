## ADDED Requirements

### Requirement: Declared environment overrides

The library SHALL let callers declare, per directory class, one environment variable name plus whether its value is an exact directory or a base for layout composition. Declaring the same class twice SHALL be a typed `DuplicateEnvDeclaration` refusal; a variable name that is empty or contains `=` or NUL SHALL be a typed `InvalidEnvDeclaration` refusal. Both checks run when the declared environment is captured: once per single-class resolve for the resolved class, or once per family resolve for the gap kinds only (after `Some` validation and the all-filled short-circuit, never per nested gap fill). Values SHALL be read with native preservation (`var_os`, never lossy conversion), so non-UTF8 values survive byte-identically. An unset or zero-length value reads as absent and falls through; any other value — including whitespace-only — validates exactly like its explicit counterpart (exact-directory declarations like the explicit tier, base declarations like an explicit base: absolute and non-empty, Windows ambient-dependent shapes refused, relative values joined against `start`, typed error when relative with no start) and reports `AppvarDirectory` or `AppvarBase` provenance respectively. Precedence SHALL be caller-supplied explicit root → declared environment value → discovery/development root → platform-standard root; a `PartialRoots::Some` value beats the environment for its kind. A caller-supplied explicit root for the resolved class masks that class's declared variable entirely — the value is never read, so a malformed value is silent — while declaration errors for a captured class still surface. Declared variables for classes a resolve does not touch SHALL NOT be read. Returned environment roots are literal candidates with no expansion, like every other tier.

#### Scenario: Declared exact directory supplies the root

- **WHEN** a caller declares `State` ← `LITRPG_STATE_DIRECTORY` as an exact directory, sets it to an absolute directory, and resolves with no explicit root
- **THEN** the library returns that directory with `AppvarDirectory` provenance without consulting discovery or platform.

#### Scenario: Declared base composes layout

- **WHEN** a caller declares a base variable (for example a `--prefix`-style root) holding an absolute directory
- **THEN** layout composition beneath it behaves exactly as for an explicit base, reported with `AppvarBase` provenance.

#### Scenario: Explicit beats environment

- **WHEN** a caller passes an explicit root and the declared variable is also set
- **THEN** the library returns the explicit root with `ExplicitDirectory` provenance.

#### Scenario: Explicit masks a malformed same-kind value

- **WHEN** a caller passes an explicit root while the declared variable for that class holds a malformed value
- **THEN** the explicit root wins with no error, because the declared value is never read.

#### Scenario: Unset or empty environment falls through

- **WHEN** a declared variable is unset or holds a zero-length value
- **THEN** resolution proceeds as if undeclared, reaching discovery or platform. (A whitespace-only value is non-empty and validates as a relative path.)

#### Scenario: Invalid environment value is a typed refusal

- **WHEN** a declared variable holds a relative value with no `start`, or a Windows ambient-dependent shape
- **THEN** the library returns the same typed refusal the corresponding explicit shape returns.

#### Scenario: Invalid declaration is refused

- **WHEN** a caller declares an empty or `=`-containing variable name, or declares the same class twice
- **THEN** capture returns `InvalidEnvDeclaration` or `DuplicateEnvDeclaration` before any tier resolves.

#### Scenario: Non-UTF8 values survive natively

- **WHEN** a declared variable holds a non-UTF8 absolute path
- **THEN** the returned root preserves its bytes exactly with environment provenance.

#### Scenario: Undeclared classes are untouched

- **WHEN** a caller declares an override for one class and resolves another
- **THEN** the other class resolves with byte-identical paths to a resolution without any declaration, and the other class's variables are never read.
