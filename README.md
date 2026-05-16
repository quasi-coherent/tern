# tern

> **A bilingual Rust framework for managing migrations targeting a PostgreSQL, MySQL, or SQLite backend.**

[![Build status](https://github.com/quasi-coherent/tern/actions/workflows/main.yaml/badge.svg?branch=master)](https://github.com/quasi-coherent/tern/actions/workflows/main.yaml)
[![Crates.io](https://img.shields.io/crates/v/tern)](https://github.com/quasi-coherent/tern)
[![Documentation](https://docs.rs/tern/badge.svg)][tern-docs]

## High level features

- Integrate an existing application with its migrations: the app and migration source can be embedded in
  a binary target.  Derive macros make this a minimum of effort.
- Migration logic can be written in SQL or Rust.  Rust migrations get a user-defined context to build the
  query at runtime.  Certain use cases are vastly simplified.
- Control precisely what runs in a database transaction and what cannot.  Control where the migration set
  stores its history, which makes it possible for multiple, independent migration sets to share the same
  database (e.g., for schema-level service isolation in a microservice setup).
- Migration sets can be an up/down style or not.
- A CLI by flipping a Cargo feature.

See this [simple][eg] example of a `tern` app.  More detailed information about this crate and its usage
can be found in the [crate documentation][tern-docs].

### As a nix flake

Additional quality-of-life things are for nix users:
- Builds are cached for the public. Append the substituter `https://quasi-coherent.cachix.org` to use it.
- `flakeModule` hiding the boilerplate of nix-ifying a `tern` app.
- `nixosConfigurations` with a MySQL, PostgreSQL, or SQLite database and a package output for deploying
  it to a VM for local integration testing, and pre-baking `tern` commands for deploying migrations to the
  VM.
- Flake template with a skeleton setup of the above.

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
[tern-wiki]: https://en.wikipedia.org/wiki/Elegant_tern
[`3.1.x`]: https://github.com/quasi-coherent/tern/tree/v3.1.x
