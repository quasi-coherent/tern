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

`tern` is a database migration library, application, and CLI supporting
embedded migrations written in SQL or Rust.

- Migrations are embedded, compiled into the final binary.
- Migrations can be written in pure SQL or built dynamically in Rust with an
  arbitrary context.
- Supports PostgreSQL, MySQL, and SQLite, and is compatible with any choice
  of database client crate.
- Allows running migration queries outside of a database transaction.
- Permits custom naming of the history table, so that it can manage multiple,
  concurrent migration sets in the same database.

## Usage

For more in-depth usage, see the [examples][examples-repo] directory.

### Quickstart

All together, a `tern` migration application looks something like this.

```rust
use tern::{MigrationContext, Tern};
use tern::executor::SqlxPgExecutor;

/// `$CARGO_MANIFEST_DIR/src/migrations` is a collection of migration files.
/// The optional `table` attribute permits a custom location for a migration
/// history table in the target database.  By default, the table
/// `_tern_migrations` is created to track history.
#[derive(MigrationContext)]
#[tern(source = "src/migrations", table = "example_history")]
struct Example {
    /// A `MigrationContext` has an associated "executor" type, which is
    /// essentially the database connection type.  The macro attribute here
    /// instructs where to find this executor.  Without it, the type itself
    /// would need to implement the executor interface.
    #[tern(executor_via)]
    executor: SqlxPgExecutor,
}

async fn main() {
    let executor = SqlxPgExecutor::new("postgres://user@localhost")
        .await
        .unwrap();
    let context = Example { executor };

    // `Tern` is the main application.  It is built from a context
    // and it uses it in the various operations that are supported.
    let app = Tern::builder()
        .apply()
        .build_with_context(context);

    // `report` is a pretty-printable array of the migrations that were
    // applied.
    let Ok(Some(report)) = app.run().await else {
        println!("error!");
    };

    println!("migrations applied successfully: {report}");
}
```

### Migrations

Migrations are defined in a directory within a Rust project's source. This
directory can contain `.rs` and `.sql` files having names matching the regex
`^V(\d+)__(\w+)\.(sql|rs)$`, e.g., `V13__create_a_table.sql` or
`V5__create_a_different_table.rs`.

A `tern` migration application centers on a type that derives the trait
[`MigrationContext`] for a directory of migration files.  The directory,
which is a module in a Rust project, has a simple organizational requirement.

- The type deriving `MigrationContext` needs to be defined in the module
  containing the migrations.

For example, if all the .sql and .rs migration files are in `src/migrations/`
then the `MigrationContext` type should be in either `src/migrations.rs` or
`src/migrations/mod.rs`.

One thing to note is that, since these files are embedded in the final
executable and static SQL migrations are not Rust source, any change to a SQL
migration won't force a recompilation.  This can cause confusing issues with
the derive macros when they aren't re-compiled.  To remedy, a `build.rs` file
should be put in the crate root with these contents:

```rust
fn main() {
    println!("cargo:rerun-if-changed=src/migrations/")
}
```

### Rust migrations

A migration written in Rust has two additional requirements:

- It contains a type `TernMigration` that derives [`Migration`].
- The trait [`QueryBuilder`] is implemented for `TernMigration`.

The first is an implementation detail of derive macros.  The second is of
course required because a migration needs a query to run!

```rust
use tern::Migration;
use tern::error::TernResult;
use tern::source::{Query, QueryBuilder};

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

### Database transactions

By default, a migration is ran in a database transaction.  Sometimes this is
not desirable and other times it is not even allowed.  For instance, in
PostgreSQL you cannot create an index `CONCURRENTLY` in a transaction.  The
user can opt out of this with certain annotations that `tern` understands.

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
need to include a user-defined type implementing [`clap::Args`] and
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

[`MigrationContext`]: tern_core::context::MigrationContext
[examples-repo]: https://github.com/quasi-coherent/tern/tree/master/examples
[`Migration`]: tern_core::source::Migration
[`QueryBuilder`]: tern_core::source::QueryBuilder
[`clap::Args`]: https://docs.rs/clap/4.5.45/clap/trait.Args.html
[`ConnectOptions`]: tern_cli::ConnectOptions

<!-- cargo-rdme end -->

## Minimum supported Rust version

`tern`'s MSRV is 1.81.0.

## Licence

This project is licensed under either of:
* MIT license ([LICENSE-MIT](./LICENSE-MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE)).
