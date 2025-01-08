<!-- cargo-rdme start -->

# tern

A database migration library and CLI supporting embedded migrations written
in SQL or Rust.

It aims to support static SQL migration sets, but expands to work with
migration queries written in Rust that are either statically determined or
that need to be dynamically built at the time of being applied.  It also aims
to do this while being agnostic to the particular choice of crate for
database interaction.

## Usage

Embedded migrations are prepared, built, and ran from a directory living in
a Rust project's source. These stages are handled by three separate traits,
but implementing any of them is generally not necessary.  `tern` exposes
derive macros that supply everything needed:

* [`MigrationSource`]: Given the required `source` macro attribute, which is
  a path to the directory containing the migrations, it prepares the
  migration set that is required of the given operation requested.
* [`MigrationContext`]: Generates what is needed of the context to be an
  acceptable type used in the [`Runner`].  It has the field attribute
  `executor_via` that can decorate a field of the struct that has some
  `Executor`, or connection type.  The context can build migration queries
  and run them.

Put together, that looks like this.

```rust
use tern::executor::SqlxPgExecutor;
use tern::{MigrationSource, MigrationContext, Runner};

/// `$CARGO_MANIFEST_DIR/src/migrations` is a collection of migration files.
/// The optional `table` attribute permits a custom location for a migration
/// history table.
#[derive(MigrationSource, MigrationContext)]
#[tern(source = "src/migrations", table = "example")]
struct Example {
   #[tern(executor_via)]
    executor: SqlxPgExecutor,
}

let executor = SqlxPgExecutor::new("postgres://user@localhost").await.unwrap();
let context = Example { executor };
let mut runner = Runner::new(context);
let report: tern::Report = runner.apply_all().await.unwrap();
println!("{report:#?}");

```

For a more in-depth example, including how a Rust migration is constructed, see
the [examples][examples-repo].

## Executors

The executor is the type responsible for actually connecting to a database
and issuing queries.  Right now, this project supports all of the `sqlx`
connection pool types via the generic [`Pool`][sqlx-pool], so that includes
PostgreSQL, MySQL, and SQLite, enabled via feature flag.  Adding other
executors is encouraged! Either in PR form or as a feature request.

## CLI

With the feature flag "cli" enabled, this exposes a CLI that can be imported
into your own migration project:

```terminal
> $ my-migration-project --help
Usage: my-migration-project <COMMAND>

Commands:
  migrate  Operations on the set of migration files
  history  Operations on the table storing the history of these migrations
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
> $ my-migration-project migrate --help
Usage: my-migration-project migrate <COMMAND>

Commands:
  apply-all     Run any available unapplied migrations
  list-applied  List previously applied migrations
  new           Create a new migration with an auto-selected version and the given description
  help          Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

[examples-repo]: https://github.com/quasi-coherent/tern/tree/master/examples
[sqlx-pool]: https://docs.rs/sqlx/0.8.3/sqlx/struct.Pool.html
[mit-license]: ./LICENSE-MIT
[apache-license]: ./LICENSE-APACHE

<!-- cargo-rdme end -->

## Minimum supported Rust version

`tern`'s MSRV is 1.81.0.

## Licence

This project is licensed under either of:
* MIT license ([LICENSE-MIT](./LICENSE-MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE)).
