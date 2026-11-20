# approots

[![Tests Status](https://github.com/emcd/approots/actions/workflows/tester.yaml/badge.svg?branch=master&event=push)](https://github.com/emcd/approots/actions/workflows/tester.yaml)
[![Project License](https://img.shields.io/badge/license-Apache--2.0-blue)](https://spdx.org/licenses/Apache-2.0.html)
[![Package Version](https://img.shields.io/crates/v/approots)](https://crates.io/crates/approots/)
[![Documentation](https://docs.rs/approots/badge.svg)](https://docs.rs/approots/)

The `approots` library answers one question — *where should this application
keep a given directory?* It resolves platform-standard locations on Linux,
macOS, and Windows. It optionally accepts explicit paths, environment
variables, and bases discovered from version control and development markers.
The library only resolves candidate directories: it never creates, modifies, or
deletes anything on the filesystem. Custom appendages can be joined to resolved
paths.

Followed conventions and standards include:
- [Apple file-system conventions](https://developer.apple.com/documentation/foundation/using-the-file-system-effectively)
- [Windows Known Folders](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid)
- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
<!-- TODO: Linux FHS, etc... for site-wide directories. -->

## Introduction

A **directory class** is a kind of application-related directory, such as
`Cache`, `Configuration`, `Data`, or `State`. Every resolution asks for one
class at a time (or one of each, through a whole-family call), and the class
decides which conventional location applies.

**Provenance** is what determined a particular directory from the resolver. For
example, it can be `ExplicitDirectory` (the caller passed a path),
`AppvarDirectory` (a declared environment variable supplied it), `Discovery` (a
discoverer found the base it was composed under), or `DefaultUserDirectory` (a
computed platform default). Every returned directory carries exactly one
provenance value, so callers can match on source instead of re-deriving it.

Resolution follows one precedence chain. The first tier that yields a
directory wins:

| Tier | When it participates | Provenance reported |
|------|----------------------|---------------------|
| Explicit argument | the caller passes a path | `ExplicitDirectory` |
| Declared environment variable, exact directory | the caller declared a variable holding a directory, and it is set | `AppvarDirectory` |
| Declared environment variable, layout base | the caller declared a variable holding a base, and it is set | `AppvarBase` |
| Discovery | a discoverer is registered | `Discovery` |
| Platform default | always, as the fallback | `SysvarUserDirectory` or `DefaultUserDirectory` |

Only the platform tier is unconditional; every other tier is opt-in — pass an
argument, declare a variable, register a discoverer, or leave the tier out
entirely. The table reads top-down: where both environment shapes are
declared, an exact directory outranks a base-derived directory.

## Features

- Platform directories: standard per-OS locations on Linux, macOS,
  and Windows, reporting whether each came from the environment or a
  default.
- Opt-in discovery: Git support is built in, and discovery is
  generic — Mercurial or proprietary systems implement the same
  discoverer interface. Nothing is ever searched unless you opt in.
- Opt-in environment overrides: declare one variable per class and the
  library reads, validates, and reports it like any other tier.
- Ordered directory bases: supply several candidate bases as
  one closed list and resolve them in precedence order. An
  unreadable entry is reported as an error rather than silently
  skipped.
- Validated subpath appendages: derive `logs/<op-id>`-style
  subdirectories beneath any resolved directory through one checked
  join.

Custom layouts by directory class are supported; see
[local structures](documentation/examples/local-structures.md).

## Installation

Add the crate to your project:

```sh
cargo add approots
```

## Usage

`Resolver` follows the builder pattern: each option returns a configured
resolver, ending in `resolve` for one class or `resolve_partially_explicit` for
the whole family at once.

```rust
use std::path::Path;
use approots::{
    DevfileDiscovererCargo, DirectoryClass, Resolver, VcsDiscovererGit,
};

// The builder accumulates one configured resolver.
let resolved = Resolver::new( "myapp" )? // validates the application name
    .register_vcs( VcsDiscovererGit )? // opt in to Git-based discovery
    .register_devfile( DevfileDiscovererCargo )? // validate discovered Cargo roots
    // Declare a State override looked up before discovery.
    .env_directory( DirectoryClass::State, "MYAPP_STATE_DIRECTORY" )
    .resolve(
        DirectoryClass::State, // the class to resolve
        // No explicit path: try the declared environment first,
        // then discover from the start directory below,
        // then fall back to platform-standard locations.
        None, // no caller-supplied directory for this call
        Some( Path::new( "/home/user/src/myapp" ) ), // where discovery starts
    )?;

println!( "{} {:?}", resolved.path.display( ), resolved.provenance );
```

Production use is the common case and needs no setup: with no discoverers
registered and no variables declared, the same `resolve` call goes straight to
platform-standard directories. The example above adds checkout discovery and
one environment override on top — Git and `Cargo.toml` support are included,
and other version-control systems plug into the same interface. See the
crate-level documentation (`cargo doc --open approots`) for the full contract.

## Additional Badges

[![Last Commit](https://img.shields.io/github/last-commit/emcd/approots)](https://github.com/emcd/approots/)
[![Copier](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/copier-org/copier/master/img/badge/badge-grayscale-inverted-border-orange.json)](https://github.com/copier-org/copier)
[![pre-commit](https://img.shields.io/badge/pre--commit-enabled-brightgreen?logo=pre-commit)](https://github.com/pre-commit/pre-commit)
[![Downloads](https://img.shields.io/crates/d/approots)](https://crates.io/crates/approots/)

## Contribution

Contribution to this project is welcome! However, it must follow the [code of
conduct](https://emcd.github.io/documentation-common/v1/sphinx-html/common/conduct.html)
for the project.

Please file bug reports and feature requests in the [issue
tracker](https://github.com/emcd/approots/issues) or submit
[pull requests](https://github.com/emcd/approots/pulls) to
improve the source code or documentation.

For development guidance and standards, please see the [development
guide](https://emcd.github.io/documentation-common/v1/sphinx-html/common/contribution.html).

## License

This project is licensed under the terms of the Apache-2.0 license.
