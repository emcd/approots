## ADDED Requirements

### Requirement: Validated subpath derivation

The library SHALL provide a join operation on `ResolvedRoot` appending caller-supplied path segments to the resolved path. Every segment SHALL be validated with the token grammar (non-empty; no separators, drive/prefix colon, NUL, or `.`/`..`) before any joining occurs; the first invalid segment SHALL return a typed error naming it. Segments join left to right onto the resolved path and the derived root SHALL report the base root's provenance unchanged. An empty segment list SHALL return the root unchanged.

#### Scenario: Multi-segment consumer subpaths

- **WHEN** a caller joins `profiles` onto a config root, or `logs` then `<op-id>` onto a state root
- **THEN** the result is the base path with the segments appended in order and the base provenance preserved.

#### Scenario: Invalid segments are refused by name

- **WHEN** any segment contains a separator, names `.` or `..`, carries a Windows drive/prefix shape, is empty, or contains NUL
- **THEN** the join returns a typed error naming the offending segment and no partial path is produced.

#### Scenario: Empty list is identity

- **WHEN** a caller joins an empty segment list onto any resolved root
- **THEN** the root returns unchanged.

#### Scenario: Uniform across tiers

- **WHEN** a caller joins the same segments onto platform, discovered, explicit, or family-supplied roots
- **THEN** validation, joining, and provenance behave identically in every case.
