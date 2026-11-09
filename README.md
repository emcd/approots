# approots

[![Tests Status](https://github.com/emcd/approots/actions/workflows/tester.yaml/badge.svg?branch=master&event=push)](https://github.com/emcd/approots/actions/workflows/tester.yaml)
[![Project License](https://img.shields.io/badge/license-Apache--2.0-blue)](https://spdx.org/licenses/Apache-2.0.html)
[![Package Version](https://img.shields.io/crates/v/approots)](https://crates.io/crates/approots/)
[![Documentation](https://docs.rs/approots/badge.svg)](https://docs.rs/approots/)

Directory locations for application caches, configuration, and state.

`approots` answers one question — *where should this application keep
a given directory?* — for configuration, state, cache, and data alike.
In production it resolves platform-standard locations on Linux,
macOS, and Windows; it can also take arbitrary roots you supply, or
roots discovered from version control and development markers, with
your own directory structures beneath them. A path you supply always
wins. Every answer says where it came from. The library only
resolves candidate paths: it never creates, modifies, or deletes
anything on the filesystem.

Key features:

- Platform directories: standard per-OS locations on Linux, macOS,
  and Windows, reporting whether each came from the environment or a
  default.
- Checkout-aware development directories: hand it any directory
  inside your project and it finds the repository root — through
  linked worktrees and nested invocations — checks the project
  identity, and builds the project's local directories under the
  right root, shared or per-worktree.
- Bring your own version control: Git support is built in, and
  discovery is generic — Mercurial or proprietary systems implement
  the same discoverer interface.
- Your layout, your default: directories follow a conventional
  layout out of the box — rooted at `.auxiliary` relative to the
  base, whether that base is one you supplied or one a discoverer
  found — and you can supply your own structure when the default
  doesn't fit.
- Layered configuration: ordered lists of configuration directories
  where the first match wins; an unreadable entry is reported as an
  error rather than silently skipped.
- Explicit override: a path you supply always wins, and no discovery
  runs when it does.

## Installation

Add the crate to your project:

```sh
cargo add approots
```

## Usage

```rust
use std::path::Path;
use approots::{
    DevfileDiscovererCargo, DirectoryClass, Resolver, VcsDiscovererGit,
};

let resolved = Resolver::new( "myapp" )?
    .register_vcs( VcsDiscovererGit )?
    .register_devfile( DevfileDiscovererCargo )?
    .resolve(
        DirectoryClass::State,
        // No explicit root: discover from the start directory below,
        // then fall back to platform-standard locations.
        None,
        Some( Path::new( "/home/user/src/myapp" ) ),
    )?;

println!( "{} {:?}", resolved.path.display( ), resolved.provenance );
```

Production use is the common case and needs no setup: with no
discoverers registered, the same `resolve` call goes straight to
platform-standard directories. The example above adds checkout
discovery on top — Git and `Cargo.toml` support are included, and
other version-control systems plug into the same interface. See the
crate-level documentation (`cargo doc --open approots`) for the full
contract.

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
