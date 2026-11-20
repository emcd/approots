# Advanced examples: local structures

This page covers `LocalStructure` composition in detail. It is
advanced material: most callers only need `Resolver::resolve`,
which applies the default structure automatically wherever a
structure applies at all.

## The default layout

A `LocalStructure` is a validated segment scheme, not a fixed
directory tree. The default composes, for the state class of an
application named `myapp` on a base `<base>`:

```text
<base>/.auxiliary/state/myapp
```

The `.auxiliary` prefix and the per-class segments
(`configuration`, `state`, `cache`, `data`) are a starting point
that suits the author's projects — not an industry convention. Treat
them as defaults to override, not as names to depend on.

## Custom structures

Every segment is a public field, so a custom layout is a struct
literal:

```rust
use std::path::PathBuf;
use approots::LocalStructure;

let structure = LocalStructure {
    auxiliary: PathBuf::from(".local"),
    state: PathBuf::from("game-state"),
    ..Default::default()
};
```

Pass it with `Resolver::local_structure`. Segments must be relative
component paths with no rooted, prefixed, or traversal components;
anything else is a typed refusal.

## Which bases a structure mates onto

`local_root` composes a structure onto any absolute base and
inherits that base's provenance unchanged: a supplied base, a
discovered base, or a declared environment base. Two exclusions:

- A `platform_root` result is already complete and namespaced; it is
  never a `local_root` base. Platform per-user directories never get
  a structure composed onto them.
- An exact directory (`ExplicitDirectory`, `AppvarDirectory`, or a
  `PartialRoots::Some` value) is used literally and bypasses
  composition entirely.

Bases are composed lexically and never canonicalized. A base that
already ends in the application name still composes exactly once
relative to itself — rawness is the caller's to track, not the
library's to detect. Nothing is ever created: the returned paths
are candidates whether or not they exist.
