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
in SQL or Rust.

It obviously aims to support plain SQL migrations, but expands to work with
migration queries written in Rust that possibly need to be built at the time
of being applied, while being agnostic to the particular choice of crate for
database interaction.

## Executors

The `Executor` trait defines methods representing the minimum required to
connect to a database, run migration queries, and interact with the migration
history table.  Right now, `tern` supports all of the [`sqlx`][sqlx-repo]
pool types via [`Pool`][sqlx-pool], which includes PostgreSQL, MySQL, and
SQLite. These can be enabled via feature flag.

### Contributing

Supporting more third-party crates would definitely be nice!  If one you like
is not available here, please feel free to contribute, either with a PR or
feature request.  Adding a new executor seems like it should not be hard.
Outside of additional backend types, we are very open and happy to hear
suggestions for how the project could be improved; simply open an issue with
the label "enhancement".  Defects or general issues detracting from usage
should also be reported in an issue, and it will be addressed immediately.

## Usage

Migrations are defined in a directory within a Rust project's source.
This directory can contain `.rs` and `.sql` files having names matching the
regex `^V(\d+)__(\w+)\.(sql|rs)$`, e.g., `V13__create_a_table.sql` or
`V5__create_a_different_table.rs`.

An operation over these migrations involves validating the entire set,
collecting the ones that are needed, resolving queries having a runtime
dependency on a user-defined context if necessary, and then performing the
operation in order.  This workflow goes by appealing to the following derive
macros:

* `MigrationSource`: Collects the migrations for use in the operation by
  parsing the directory into a validated, prepared, sorted, and uniform
  collection, and exposing methods to return a given subset.
* `MigrationContext`: A type providing context to perform the operation
  on the migrations passed from `MigrationSource` given some parameters.

Put together, it looks like this.

```rust
use tern::{MigrationSource, MigrationContext, Runner, SqlxPgExecutor};

/// `$CARGO_MANIFEST_DIR/src/migrations` is a collection of migration files.
/// The optional `table` attribute permits a custom location for a migration
/// history table in the target database.
#[derive(MigrationSource, MigrationContext)]
#[tern(source = "src/migrations", table = "example")]
struct Example {
   // `Example` itself needs to be an executor without this annotation.
   #[tern(executor_via)]
    executor: SqlxPgExecutor,
}

let executor = SqlxPgExecutor::new("postgres://user@localhost").await.unwrap();
let context = Example { executor };
let mut runner = Runner::new(context);
let report: tern::Report = runner.run_apply_all(false).await.unwrap();
println!("migration run report: {report}");
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
arbitrary migration context to flexibly build the query at runtime.  For
this to work, `MigrationSource` and `MigrationContext` get us nearly there,
but the user needs to follow a couple rules and write a simple implementation
of a trait to complete the requirements.

The first rule is that the type deriving `MigrationSource` needs to be
declared in the immediate parent module of the migrations.  So if
`source = "src/migrations"`, a perfect place to put the type that derives
`MigrationContext` and `MigrationSource` is in `src/migrations.rs`, which
would have to exist in any case. Depending on preference, the module
`migrations` can also be a `mod.rs` living next to the migrations. This is
required because ultimately each migration defines a module and we need to
reference that module and members of it when expanding syntax.  The simplest
way is to assume this module hierarchy.

The other requirement is that there needs to be a struct that is called
`TernMigration` in any Rust migration source file, and that it derives a
third macro, `Migration`.  `Migration` doesn't contribute much, but the
struct deriving it is still needed because of another implementation detail
of the macros: we need a way for the `Migration` macro to share data with the
other two macros, the alternative being to parse the entire token stream of a
Rust source file in the course of `MigrationSource` performing its duties,
clearly a less appealing option.

This `TernMigration` is needed because the last thing that's required is a
user-defined method on it that provides instructions for how to build its
query using the migration context.

```rust
use tern::{Query, QueryBuilder, Migration};
use tern::error::TernResult;

use super::Example;

/// Use the optional macro attribute `#[tern(no_transaction)]` to avoid
/// running this in a database transaction.
#[derive(Migration)]
pub struct TernMigration;

impl QueryBuilder for TernMigration {
    /// The custom-defined migration context.
    type Ctx = Example;

    /// When `await`ed, this should produce a valid SQL query wrapped by
    /// `Query`.  This is what will run against the database.
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
in practice and introduce problems just as they solve others. The section
"Important Notes" in [this][flyway-undo] flyway documentation summarizes our
feelings well.

## Database transactions

By default, a migration and its accompanying schema history table update are
ran together in a database transaction.  Sometimes this is not desirable and
other times it is not even allowed.  For instance, in postgres you cannot
create an index `CONCURRENTLY` in a transaction.  To give the user the option,
`tern` understands certain annotations and will not run that migration in a
database transaction if they are present.

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

With the feature flag "cli" enabled the type `App` is exported, which is a
CLI wrapping `Runner` methods that can be imported into your own migration
project to turn it into a CLI, provided that the migration context has an
implementation of `ContextOptions` which simply says how to create the
context from a connection string.

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
Operations on the set of migration files
Usage: my-migration-project migrate <COMMAND>

Commands:
  apply         Run the apply operation for a specific range of unapplied migrations
  apply-all     Run any available unapplied migrations
  soft-apply    Insert migrations into the history table without applying them
  list-applied  List previously applied migrations
  new           Create a migration with the description and an auto-selected version
  help          Print this message or the help of the given subcommand(s)

Options:
-h, --help  Print help

> $ my-migration-project history --help
Operations on the table storing the history of these migrations

Usage: my-migration-project history <COMMAND>

Commands:
  init        Create the schema history table
  drop        Drop the schema history table
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

[examples-repo]: https://github.com/quasi-coherent/tern/tree/master/examples
[sqlx-repo]: https://github.com/launchbadge/sqlx
[sqlx-pool]: https://docs.rs/sqlx/0.8.3/sqlx/struct.Pool.html
[flyway-undo]: https://documentation.red-gate.com/fd/migrations-184127470.html#Migrations-UndoMigrations

<!-- cargo-rdme end -->

## Minimum supported Rust version

`tern`'s MSRV is 1.81.0.

## Licence

This project is licensed under either of:
* MIT license ([LICENSE-MIT](./LICENSE-MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE)).
