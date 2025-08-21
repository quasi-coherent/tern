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

<!-- cargo-rdme start -->

A database migration library and CLI supporting embedded migrations written
in SQL or Rust, compatible with any underlying third-party database crate.

## Usage

Migrations are defined in a directory within a Rust project's source. This
directory can contain `.rs` and `.sql` files having names matching the regex
`^V(\d+)__(\w+)\.(sql|rs)$`, e.g., `V13__create_a_table.sql` or
`V5__create_a_different_table.rs`.

The `Tern` type is the main application for performing an
operation given a set of source migrations, but it needs a user-defined
[`MigrationContext`] to be built.  This crate provides a derive macro for
`MigrationContext`.

Put together, it looks like this:

```rust
use tern::{MigrationContext, Tern};
use tern::executor::SqlxPgExecutor;

/// `$CARGO_MANIFEST_DIR/src/migrations` is a collection of migration files.
/// The optional `table` attribute permits a custom location for a migration
/// history table in the target database.
#[derive(MigrationContext)]
#[tern(source = "src/migrations", table = "example_history")]
struct Example {
    /// A `MigrationContext` has an associated type that implements
    /// `tern::context::Executor` and this macro attribute points to
    /// what should be used for that.  Without it, the type itself must
    /// implement `Executor`.
    #[tern(executor_via)]
    executor: SqlxPgExecutor,
}

async fn main() {
    let executor = SqlxPgExecutor::new("postgres://user@localhost")
        .await
        .unwrap();
    let context = Example { executor };
    let app = Tern::builder()
        .apply()
        .build_with_context(context);

    let Ok(Some(report)) = app.run().await else {
        println!("error: {e}");
    };

    println!("migrations applied successfully: {report}");
}
```

For more in-depth examples, see the [examples][examples-repo].

## SQL migrations

Since migrations are embedded in the final executable and static SQL
migrations are not Rust source, any change to a SQL migration won't force
a recompilation.  The proc macro that parses these files will then not be
up-to-date, and this can cause confusing issues.  To remedy, a `build.rs`
file should be put in the crate root with these contents:

```rust
fn main() {
    println!("cargo:rerun-if-changed=src/migrations/")
}
```

## Rust migrations

Migrations can be written in Rust, and these can take advantage of the
migration context to flexibly build the query at runtime.  All migrations,
SQL or Rust, are required to implement [`Migration`].  This is automatic for
migrations written in pure SQL, but for one written in Rust an implementation
of [`QueryBuilder`] needs to be supplied.  This simply demonstrates how the
query for this migration is defined.

```rust
use tern::Migration;
use tern::error::TernResult;
use tern::source::{Query, QueryBuilder};

// The `MigrationContext` for this migration set should be defined in the
// parent module of the migration source directory.  It is an implementation
// detail of the derive macro for `MigrationContext` that it is able to
// declare a module for each migration, which will contain the `Migration`
// implementation.
use super::Example;

/// Use the optional macro attribute `#[tern(no_transaction)]` to avoid
/// running this query in a database transaction.
#[derive(Migration)]
pub struct TernMigration;

impl QueryBuilder for TernMigration {
    /// The custom-defined migration context.
    type Ctx = Example;

    /// When `await`ed, this should produce a valid SQL query.
    async fn build(&self, ctx: &mut Self::Ctx) -> TernResult<Query> {
        // Really anything can happen here.  It just depends on what
        // `Self::Ctx` can do.
        let sql = "SELECT 1;";
        let query = Query::new(sql);

        Ok(query)
    }
}
```

## Reversible migrations

As of now, the official stance is to not support an up-down style of
migration set, the philosophy being that down migrations are not that useful
in practice and introduce problems just as they solve others.

## Database transactions

By default, a migration is ran in a database transaction.  Sometimes this is
not desirable and other times it is not even allowed.  For instance, in
postgres you cannot create an index `CONCURRENTLY` in a transaction.  To give
the user the option, `tern` understands certain annotations and will not run
that migration in a database transaction if they are present.

For a SQL migration:

```sql
-- tern:noTransaction is the annotation for SQL.  It needs to be found
-- somewhere on the first line of the file.
CREATE INDEX CONCURRENTLY IF NOT EXISTS blah ON whatever;
```

For a Rust migration:

```rust
use tern::Migration;

/// Don't run this in a transaction.
#[derive(Migration)]
#[tern(no_transaction)]
pub struct TernMigration;
```

## CLI

When the feature flag "cli" is enabled, `Tern` exposes a
method `run_cli` that packages the same
operations and options with a command line argument parser.  The arguments
include a user-defined type implementing [`clap::Args`] and
[`ConnectOptions`] for initializing the migration context.

```rust
use tern::cli::clap::{self, Args};
use tern::error::{Error, TernResult};
use tern::executor::SqlxPgExecutor;
use tern::{ConnectOptions, MigrationContext, Tern};

#[derive(MigrationContext)]
#[tern(source = "src/migrations", table = "example_history")]
struct Example {
   // `Example` itself needs to be an executor without this annotation.
   #[tern(executor_via)]
    executor: SqlxPgExecutor,
}

// Custom CLI argument(s) required to build this context.
#[derive(Debug, Args)]
struct ExampleOptions {
    /// Connection string
    #[clap(long, short = 'D', env)]
    db_url: Option<String>,
}

impl ConnectOptions for ExampleOptions {
    type Ctx = Example;

    async fn connect(&self) -> TernResult<Example> {
       let db_url = self
           .db_url
           .as_deref()
           .ok_or_else(|| Error::Init("missing db connection string".into()))?;
        let executor = SqlxPgExecutor::new(db_url).await?

        Ok(Example { executor })
    }
}

async fn main() {
    // Operation and parameters are parsed as command line arguments.
    match Tern::run_cli::<ConnectExample>().await {
        Err(e) => println!("error {e}"),
        Ok(Some(report)) => println!("success: {report}"),
        _ => println!("OK"),
    }
}
```

This would be used as follows:

```terminal
> $ example --help
Usage: example <COMMAND>

Commands:
  migrate  Operations on the set of migration files
  history  Operations on the table storing the history of these migrations
  help     Print this message or the help of the given subcommand(s)

Options:
-D, --db-url <DB_URL> Connection string [env: DB_URL=]
-h, --help  Print help

> $ example migrate --help
Operations on the set of migration files
Usage: example migrate <COMMAND>

Commands:
  apply         Run the apply operation for all unapplied versions or a range of versions
  soft-apply    Insert migrations into the history table without applying them
  list-applied  List previously applied migrations
  help          Print this message or the help of the given subcommand(s)

Options:
-D, --db-url <DB_URL> Connection string [env: DB_URL=]
-h, --help  Print help
```

[sqlx-repo]: https://github.com/launchbadge/sqlx
[`MigrationContext`]: tern_core::context::MigrationContext
[examples-repo]: https://github.com/quasi-coherent/tern/tree/master/examples
[`Migration`]: tern_core::source::Migration
[`QueryBuilder`]: tern_core::source::QueryBuilder
[sqlx-pool]: https://docs.rs/sqlx/0.8.3/sqlx/struct.Pool.html
[`clap::Args`]: https://docs.rs/clap/4.5.45/clap/trait.Args.html
[`ConnectOptions`]: tern_cli::ConnectOptions

<!-- cargo-rdme end -->

## Minimum supported Rust version

`tern`'s MSRV is 1.81.0.

## Licence

This project is licensed under either of:
* MIT license ([LICENSE-MIT](./LICENSE-MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE)).
