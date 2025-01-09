<!-- cargo-rdme start -->

<h1 align="center">tern</h1>
<br />
<div align="center">
  <!-- Version -->
  <a href="https://crates.io/crates/tern">
    <img src="https://img.shields.io/crates/v/tern.svg?style=flat-square"
    alt="Crates.io version" /></a>
  <!-- Docs -->
  <a href="https://docs.rs/tern">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
</div>

A database migration library and CLI supporting embedded migrations written
in SQL or Rust.

It aims to support static SQL migration sets, but expands to work with
migration queries written in Rust that are either statically determined or
that need to be dynamically built at the time of being applied.  It also aims
to do this while being agnostic to the particular choice of crate for
database interaction.

## Executors

The abstract [`Executor`] is the type responsible for actually connecting to
a database and issuing queries.  Right now, this project supports all of the
[`sqlx`][sqlx-repo] pool types via the generic [`Pool`][sqlx-pool], so that
includes PostgreSQL, MySQL, and SQLite. These can be enabled via feature
flag.

Adding more executors is welcomed! That can be in PR form or as a feature
request.  Adding an executor seems like it should not be hard.

## Usage

Embedded migrations are prepared, built, and ran off a directory living in
a Rust project's source. These stages are handled by three separate traits,
but implementing any of them is generally not necessary --  `tern` exposes
derive macros that supply everything needed to satisfy them.

* [`MigrationSource`]: Given the required `source` macro attribute, which is
  a path to the directory containing the migrations, it prepares the
  migration set that is required of the given operation requested.
* [`MigrationContext`]: Generates what is needed of the context to be an
  acceptable type used in the [`Runner`].  It has the field attribute
  `executor_via` that can decorate a field of the struct that has some
  [`Executor`], or connection type.  The context can build migration queries
  and run them using this executor.

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
   // `Example` itself needs to be an executor without the annotation.
   #[tern(executor_via)]
    executor: SqlxPgExecutor,
}

let executor = SqlxPgExecutor::new("postgres://user@localhost").await.unwrap();
let context = Example { executor };
let mut runner = Runner::new(context);
let report: tern::Report = runner.apply_all().await.unwrap();
println!("{report:#?}");

```

For a more in-depth example, see the [examples][examples-repo].

### SQL migrations

Since migrations are embedded in the final executable, and static SQL
migrations are not Rust source, any change to a SQL migration doesn't force
a recompilation, which can cause confusing issues.  To remedy, a `build.rs`
file can be put in the project directory with these contents:

```rust
fn main() -> {
    println!("cargo:rerun-if-changed=src/migrations/")
}
```

### Rust migrations

Migrations can be expressed in Rust, and these can take advantage of the
arbitrary migration context to flexibly build the query at runtime.  To do
this, the context needs to know how to build the query, and what migration
to build it for.  This is achieved using some convention, a hand-written
trait implementation, and another macro:

```rust
use tern::error::TernResult;
use tern::migration::{Query, QueryBuilder};
use tern::Migration;

/// This is the convention: it needs to be called this for the macro
/// implementation.  This macro has an attribute `no_transaction` that
/// instructs the context associated to it by `QueryBuilder` below to run
/// the query outside of a transaction.
#[derive(Migration)]
pub struct TernMigration;

impl QueryBuilder for TernMigration {
    /// The context from above.
    type Ctx = Example;

    async fn build(&self, ctx: &mut Self::Ctx) -> TernResult<Query> {
        // Really anything can happen here.  It just depends on what
        // `Self::Ctx` can do.
        let sql = "SELECT 1;";
        let query = Query::new(sql);
        Ok(query)
    }
}
```

### Database transactions

By default, a migration and its accompanying schema history table update run
in a database transaction.  Sometimes this is not desirable and other times
it is not allowed.  For instance, in postgres you cannot create an index
`CONCURRENTLY` in a transaction.  To give the user the option, `tern` reads
certain annotations to determine whether the runner should apply the
migration in a transaction or not.

For a SQL migration:

```sql
-- tern:noTransaction
-- This annotation has to be on the first line after `--`
CREATE INDEX CONCURRENTLY IF NOT EXISTS blah ON whatever;
```

For a Rust migration:

```rust
use tern::Migration;

/// Don't run this in a migration.
#[derive(Migration)]
#[tern(no_transaction)]
pub struct TernMigration;
```

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
[`MigrationSource`]: https://docs.rs/tern/1.0.1/tern/trait.MigrationSource.html
[`MigrationContext`]: https://docs.rs/tern/1.0.1/tern/trait.MigrationContext.html
[`Executor`]: https://docs.rs/tern/1.0.1/tern/trait.Executor.html
[`Runner`]: https://docs.rs/tern/1.0.1/tern/struct.Runner.html
[examples-repo]: https://github.com/quasi-coherent/tern/tree/master/examples
[sqlx-repo]: https://github.com/launchbadge/sqlx
[sqlx-pool]: https://docs.rs/sqlx/0.8.3/sqlx/struct.Pool.html

<!-- cargo-rdme end -->

## Minimum supported Rust version

`tern`'s MSRV is 1.81.0.

## Licence

This project is licensed under either of:
* MIT license ([LICENSE-MIT](./LICENSE-MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE)).
