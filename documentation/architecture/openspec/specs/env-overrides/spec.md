# env-overrides

## Purpose

Opt-in per-class environment variable declarations that the library reads as override roots, with shape-aware validation, single-read capture, and environment provenance.

## Requirements

### Requirement: Declared environment overrides

The library SHALL let callers declare, per directory class, one environment variable name per shape: at most one exact directory and at most one layout base. Declaring the same shape twice for one class SHALL be a typed `DuplicateEnvDeclaration` refusal; a variable name that is empty or contains `=` or NUL SHALL be a typed `InvalidEnvDeclaration` refusal. A single class MAY declare both shapes, including the same variable name for both. Both declaration checks run when the declared environment is captured: once per single-class resolve for the resolved class, or once per family resolve for the gap kinds only (after `Some` validation and the all-filled short-circuit, never per nested gap fill). Value validation runs afterwards, at selection time before any tier is chosen: both present values validate even when only one can win. Values SHALL be read once per distinct variable name with native preservation (`var_os`, never lossy conversion), and the cached value (including absence) SHALL be reused for every class declaring that name; per-class shape validation and provenance still apply independently. Cache keys are the exact declared names. Windows may resolve case aliases to the same OS variable, but different declared spellings are separate cache keys and separate reads; no equality guarantee across reads under mutation. One read per name per capture is guaranteed; an atomic process-wide snapshot is not claimed. Non-UTF8 values survive byte-identically through the native read. An unset variable reads as absent and falls through; a zero-length value is malformed and returns the explicit tier's `EmptyPath` refusal, so an accidental `VAR=` (for example from an unset shell expansion without `set -u`) fails loudly instead of silently falling through to another tier. Any other value — including whitespace-only — validates exactly like its explicit counterpart (exact-directory declarations like the explicit tier, base declarations like an explicit base: absolute and non-empty, Windows ambient-dependent shapes refused, relative values joined against `start`, typed error when relative with no start) and reports `AppvarDirectory` or `AppvarBase` provenance respectively. Precedence SHALL be caller-supplied explicit root → declared exact directory → declared layout base → discovery/development root → platform-standard root; a `PartialRoots::Some` value beats the environment for its kind. A caller-supplied explicit root for the resolved class masks that class's declared variables entirely — no value and no declaration is read. Declared variables for classes a resolve does not touch SHALL NOT be read. The same variable name may back multiple classes: it is read once, and the cached value (including absence) is reused for every class declaring that name. Returned environment roots are literal candidates with no expansion, like every other tier.

#### Scenario: Declared exact directory supplies the root

- **WHEN** a caller declares `State` ← `LITRPG_STATE_DIRECTORY` as an exact directory, sets it to an absolute directory, and resolves with no explicit root
- **THEN** the library returns that directory with `AppvarDirectory` provenance without consulting discovery or platform.

#### Scenario: Declared base composes layout

- **WHEN** a caller declares a base variable (for example a `--prefix`-style root) holding an absolute directory
- **THEN** layout composition beneath it behaves exactly like an explicit base, reported with `AppvarBase` provenance.

#### Scenario: Exact directory beats layout base

- **WHEN** a caller declares both an exact directory and a layout base for one class, and both hold valid absolute values
- **THEN** the library returns the exact directory with `AppvarDirectory` provenance; the base is validated but not used.

#### Scenario: Invalid value of either shape is refused

- **WHEN** a caller declares both shapes for one class and either value is malformed (for example the unused base holds a relative value with no `start`)
- **THEN** the typed refusal returns instead of the winning shape's root.

#### Scenario: Explicit beats environment

- **WHEN** a caller passes an explicit root while the declared variable is also set
- **THEN** the library returns the explicit root with `ExplicitDirectory` provenance.

#### Scenario: Explicit masks a malformed same-kind value

- **WHEN** a caller passes an explicit root while the declared variable for that class holds a malformed value
- **THEN** the explicit root wins with no error, because the declared value is never read.

#### Scenario: Explicit masks declaration problems

- **WHEN** a caller passes an explicit root while the class's declaration is invalid (empty name) or duplicated
- **THEN** the explicit root wins with no error, because the declaration is never captured.

#### Scenario: Unset environment falls through

- **WHEN** neither declared variable for a class is set
- **THEN** resolution proceeds as if undeclared, reaching discovery or platform.

#### Scenario: Zero-length environment is an error, not absence

- **WHEN** a declared variable holds a zero-length value
- **THEN** the library returns the explicit tier's `EmptyPath` refusal rather than falling through. (A whitespace-only value is non-empty and validates as a relative path.)

#### Scenario: Invalid environment value is a typed refusal

- **WHEN** a declared variable holds a relative value with no `start`, or a Windows ambient-dependent shape
- **THEN** the library returns the same typed refusal the corresponding explicit shape returns.

#### Scenario: Invalid declaration is refused

- **WHEN** a caller declares an empty or `=`-containing variable name, or declares the same shape twice for one class
- **THEN** capture returns `InvalidEnvDeclaration` or `DuplicateEnvDeclaration` before any tier resolves.

#### Scenario: Non-UTF8 values survive natively

- **WHEN** a declared variable holds a non-UTF8 absolute path
- **THEN** the returned root preserves its bytes exactly with environment provenance.

#### Scenario: Shared variable supplies two classes

- **WHEN** the same variable holds a valid absolute directory and is declared for both State and Cache
- **THEN** each class resolves that directory with environment provenance from a single read of the shared name.

#### Scenario: Undeclared classes are untouched

- **WHEN** a caller declares an override for one class and resolves another
- **THEN** the other class resolves with byte-identical paths to a resolution without any declaration, and the other class's variables are never read.
