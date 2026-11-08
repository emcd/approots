# approots

[![Tests Status](https://github.com/emcd/approots/actions/workflows/tester.yaml/badge.svg?branch=master&event=push)](https://github.com/emcd/approots/actions/workflows/tester.yaml)
[![Project License](https://img.shields.io/badge/license-Apache--2.0-blue)](https://spdx.org/licenses/Apache-2.0.html)
[![Package Version](https://img.shields.io/crates/v/approots)](https://crates.io/crates/approots/)
[![Documentation](https://docs.rs/approots/badge.svg)](https://docs.rs/approots/)
Directory locations for application caches, configuration, and state.

`approots` is a small, caller-driven, read-only resolution library for
repository-aware applications. It answers one question — *where should
this application keep a given directory?* — through an explicit
precedence chain (caller-supplied explicit root, then discovered
development or supplied-directory root, then platform-standard root)
and reports which tier supplied every result. The library only resolves
candidate paths: it never creates, modifies, deletes, or locks anything
on the filesystem.

Key features:

- Explicit precedence with a closed six-value provenance vocabulary, so
  callers always know where a root came from.
- Hardened Git checkout discovery (explicit working directory,
  prefix-based environment scrubbing, typed failure outcomes) behind a
  `VcsDiscovererGit` discoverer with shared-vs-local scope selection.
- `Cargo.toml` identity validation (the sole declared exception to the
  no-parsing rule), both at a selected checkout and as an upward
  standalone search.
- `LocalStructure` mates the conventional `.auxiliary` tree onto any
  discovered or supplied base, inheriting its provenance.
- Platform-standard roots on Linux, macOS, and Windows with honest
  environment-vs-default provenance, plus shared name sanitization and
  ordered closed configuration layer lists.

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
        None,
        Some( Path::new( "/home/user/src/myapp" ) ),
    )?;

println!( "{} {:?}", resolved.path.display( ), resolved.provenance );
```

With no discoverers registered, resolution falls through to the
platform tier; with an explicit root supplied, that root wins with
`CallerProvided` provenance and no discovery runs. See the crate-level
documentation (`cargo doc --open approots`) for the full contract.

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
