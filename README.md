# tern

A bilingual Rust framework for managing migrations targeting a PostgreSQL, MySQL, or SQLite backend.

[![Build status](https://github.com/quasi-coherent/tern/actions/workflows/main.yaml/badge.svg?branch=master)](https://github.com/quasi-coherent/tern/actions/workflows/main.yaml)
[![Crates.io](https://img.shields.io/crates/v/tern)](https://github.com/quasi-coherent/tern)
[![Documentation](https://docs.rs/tern/badge.svg)][tern-docs]

More detailed information about this crate and its usage can be found in the [crate documentation][tern-docs].

## High level features

- Integrate an existing application with its migrations: the complete migration source and operations
  with it are embedded in a binary target.  Derive macros make this a minimum of effort and code change.
- Migration logic can be written in SQL or Rust.  Rust migrations get a user-defined context to build it
  and its query at runtime, vastly simplifying some types of real-world use case.
- Define a migration set as an up/down pair or not. Control whether any or all of a migration should take
  place in a database transaction.
- Control where migration history is recorded, which makes it possible to have any number of independent
  migration sets exist in the same database, well-suited for schema-level service isolation.
- Easily write comprehensive integration tests of your migration set.
- Add a CLI by simply enabling a feature.

See this [simple][eg] example of a `tern` app.

### As a nix flake

Additional quality-of-life things are given to nix users:
- This is a flake with package outputs for each crate and build dependencies cached in cachix. Append the
  substituter `https://quasi-coherent.cachix.org` to take advantage of the cache.
- `flakeModules` provide options for building and testing a `tern` project.
- Other module options exist for creating one or more [`nixos-lima`][lima] VMs, which could be used for
  setting up more elaborate integration test environments, or even for setting up _all_ environments!

## A note on a noun

This project is called `tern`.  Apparently, so are many other database migration projects.

I can only assume this means that the migratory [species][tern-wiki] has a larger portion of their budget
going to SEO services than other families of birds that are known for having extremely long migratory patterns.

It's hard to find any that can measure up to the tern though.  Recent studies establish that the Arctic tern,
for instance, covers a round-trip length of 70,000km each year, which makes me wonder if they do anything but
migrate.

## ⚠ Breaking changes ⚠

We are currently working on a 4.0.0 release, so the default branch contains many breaking changes.  See the
[`3.1.x`] branch for the current non-experimental release on crates.io.

## Contributing

Contributions in the form of PR, feature request, or bug report are all very much appreciated.  Currently, a
decent place to contribute in the feature department could be to add integrations with more third-party database
crates.  Or it could not--this author only knows `sqlx` and can't say much about the popularity of other options.

Enhancements, defects, or general issues of behavior (flaws, if you will) belong in an issue with that label.

## Licence

This project is licensed under either of:
* MIT license ([LICENSE-MIT](./LICENSE-MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE)).

[tern-docs]: https://docs.rs/tern/latest/tern/
[eg]: ./examples/simple.rs
[lima]: https://github.com/nixos-lima/nixos-lima
[tern-wiki]: https://en.wikipedia.org/wiki/Elegant_tern
[`3.1.x`]: https://github.com/quasi-coherent/tern/tree/v3.1.x
