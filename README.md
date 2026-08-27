# tern

> Bilingual database migrations for MySQL, PostgreSQL, or SQLite.

[![Build status](https://github.com/quasi-coherent/tern/actions/workflows/main.yaml/badge.svg?branch=master)](https://github.com/quasi-coherent/tern/actions/workflows/main.yaml)
[![Crates.io](https://img.shields.io/crates/v/tern)][tern-cratesio]
[![Documentation](https://docs.rs/tern/badge.svg)][tern-docsrs]

### Overview

`tern` is a Rust framework for managing a set of database migrations.  At a high level it provides a
few core features:

- A simple macro interface creating a migration app that compiles with the migration source into a
  single target.
- An API for supplying a custom "context" to build and apply migrations.
- The query defining a migration can be written in ordinary SQL or in Rust using the custom context.
- A set of macros emitting test suites of the migration set and context.
- A public API that avoids over-caution:
  * Ability to manipulate the pointer to database state more freely: "soft" apply or revert, replay,
    rewind, or rewrite portions of migration history.
  * A migration may be many statements that need not run in a transaction.
  * Multiple, independent migration sets can co-exist in the same database.
  * Extend or customize the set of migration operations; personalize how schema history is stored.

For more, check out some [examples][eg] of `tern` applications or browse the official Rust
documentation [here][tern-docsrs].

### ⚠ Breaking changes ⚠

The default branch is a major release candidate so it contains many breaking changes over the
previous release. See the [`3.1.x`] branch for the current, non-RC version of `tern`.

### A note on a noun

This project is called `tern`.  Apparently, so are many other database migration projects.

I can only assume this means that the migratory [species][tern-wiki] has a larger portion of their
budget going to SEO services than other families of birds that are known for having extremely long
migratory patterns.

It's hard to find any that can measure up to the tern though.  Recent studies establish that the
Arctic tern, for instance, covers a round-trip length of 70,000km each year, which makes me wonder
if they do anything but migrate.

## Usage

To install `tern`, you select a supported third-party database crate to bring in as a dependency
that matches your target database.  Currently, there is support for the [`sqlx`][sqlx-conn]
connection types for MySQL, PostgreSQL, and SQLite.  Add this to your Cargo.toml, for example:

```toml
tern = { version = "4.0.0-rc1", features = ["sqlx_postgres"] }
```

A `tern` application consists of two things: a migration set, or the queries representing the
version history, and a `TernApp` for exposing methods to operate on the database with them.  Both
are provided by the derive macro `TernApp`.

A "kitchen sink" example of what that looks like is:

```rust
use tern::TernApp;
use tern::executor::SqlxPgExecutor;

/// Migrations and a context to apply them.
///
/// Here `schema` and `table` drive where the database state relative to the
/// migrations should be stored.  Both are optional.  Once the table exists,
/// this obviously should not change.
///
/// The `source` attribute is the path to the migration source directory.
#[derive(Clone, Debug, TernApp)]
#[tern(source = "src/migrations", schema = "tern_history", table = "__blah")]
pub struct BlahMigrations {
    /// We need a database client that implements a particular "utility" query
    /// interface.  The cargo feature brought in `SqlxPgExecutor`, which does
    /// this.  `executor_via` points out the field to grab it from.
    ///
    /// This is technically optional, but without it `BlahMigrations` itself
    /// would need to provide the lower level database methods.
    #[tern(executor_via)]
    pub exec: SqlxPgExecutor,

    /// Whatever you desire.
    pub special_value: Option<String>,
}
```

`BlahMigrations` can now be turned into a runnable application:

```rust
use tern::executor::{ConnStr, SqlxPgExecutor, SqlxPgExecutorOptions};
use tern::ops::{ApplyArgs, ListArgs};
use tern::Tern;

// Constructing an interior "utility" database client (the "executor"):
let conn = ConnStr::from_env("DATABASE_URL")?;
let exec = SqlxPgExecutor::new(&conn).await?;

// Our `TernApp`:
let special_value = Some("lebron_james".into());
let blah = BlahMigrations { exec, special_value };

// `Tern` wraps the app and exposes a set of the "main" methods.
let mut app = Tern::new(blah);

// `List` is one such method.  It prints migrations in the migration set.
// The `diff` option returns the diff between local and remote sources, or
// equivalently, the unapplied migrations.
let list_args = ListArgs::new().diff();

let unapplied = app.list(list_args).await?;
println!("unapplied migrations: {unapplied}");

// Looks good :+1:
// The `Apply` operation with the `all` option runs all available unapplied
// migrations.
let apply_args = ApplyArgs::new().all();
match app.apply(apply_args).await {
    Ok(complete) => println!("migration complete, applied: {complete}"),
    Err(e) => println!("failed migration, partial result and error: {e}"),
}
```

### CLI

The feature `cli`, enabled by default, allows operations and their arguments to be provided in a
CLI:

```rust
use tern::TernCli;
use tern::executor::SqlxPgExecutor;

// Create the tern app `BlahMigrations` and combine with command line arguments:
let app = TernCli::try_init_with(|conn| async move {
    let exec = SqlxPgExecutor::new(&conn).await?;
    let special_value = Some("lebron_james".into());
    Ok(BlahMigrations { exec, special_value })
})
.await?;

// This runs the operation that was specified as a CLI subcommand:
match app.run().await {
    Ok(complete) => println!("operation success: {complete}"),
    Err(e) => println!("operation failed: {e}"),
}
```

### Migrations

Migrations are part of the Rust source code, located in the directory that the `source` attribute of
`TernApp` references, a path relative to `CARGO_MANIFEST_DIR`.

These files are expected to follow these conventions/rules:

* A migration file can be in Rust (more [below](#rust-migrations)) or in SQL.  The eventual output
  should be a prepared statement to send to the database, the evaluation of which is either deferred
  or done at compile time.
* A migration query can have one or more constituent parts (expressions ending with `;`).
* Migration source filenames must match the regex pattern `^(V|U|D)(\d+)__(\w+)\.(sql|rs)$`.  For
  example,
  - `V1__create_a_table.sql`
  - `V5__create_a_different_table.rs`
  - `U91__create_a_table_again.sql`
* A migration set can come in pairs, prefixed with `U` (for "up") and `D` (for "down"), when one up
  migration is ostensibly reverted by its corresponding down migration. For example,
  - `U8__create_table_index.sql` and `D8__create_table_index.sql`
  - `U22__do_a_thing.rs` and `D22__do_a_thing.sql`
* One up/down pair can be any combination of .rs and .sql.  A migration set cannot mix `V` with
  `U`/`D`-prefixed files.

If a migration set is an up/down type, additional operations for reverting the version of the
database are enabled.  More on down migrations [below](#reverting-migrations).

#### SQL annotations

We proclaimed at the top that you have control over what runs in a database transaction and what
does not.  This becomes important in some common scenarios.  For example, it is an error to try to
create an index with the `CONCURRENTLY` keyword in PostgreSQL in a transaction.

To facilitate, `tern` understands a file header directive that will prevent the migration from being
ran in a transaction, which

* `tern:noTransaction` should be found somewhere in a comment on the first line if the file should
  not be interpreted as one query to run in a transaction.
* A SQL dialect hint can be given by, e.g., `tern:postgres`.  Supported values are `mysql`,
  `postgres`, and `sqlite`.  This must be on the first line in a comment, together with
  `noTransaction` if both should be included: `tern:noTransaction,postgres`.  Rarely this will be
  useful when less common, dialect-specific syntax confuses the interpretation of the statement
  terminating character only in the rare case that dialect-specific syntax confuses the
  interpretation of the statement terminating character.  It is usually possible to just remove the
  problematic syntax.

```sql
-- tern:noTransaction
-- The file now is treated as groups of statements ran sequentially outside of a transaction.

-- Not in a transaction.
CREATE TABLE IF NOT EXISTS whatever (
  we_id serial PRIMARY KEY,
  created_at timestamptz(3) NOT NULL DEFAULT now(),
  user_id uuid NOT NULL DEFAULT uuid_generate_v1mc(),
);

-- But you can run parts of the migration file in a transaction normally:
BEGIN;
CREATE TABLE IF NOT EXISTS whatever_whatever (
  wewe_id serial PRIMARY KEY,
  we_created_at timestamptz(3) REFERENCES whatever(we_id),
  last_login_at timestamptz(3),
  user_name text,
);

CREATE INDEX IF NOT EXISTS i_do_what_i_want
  ON whatever_whatever (we_created_at);
COMMIT;

SELECT 'comme ci, comme ca';
```

#### Rust migrations

The migration source files become bona fide modules in the Rust source tree, and each is unified by
the [`Migration`](https://docs.rs/tern/latest/tern/trait.Migration.html) interface.  This is done
automatically for SQL sources, but not quite for Rust sources.

The derive macro for `Migration` will do it, but only with a little help from the user to write out
something that would need to be written out in any case:

```rust
use tern::Migration;
use tern::error::TernResult;
use tern::migration::{Query, ResolveQuery};

// This is the type deriving `TernApp` from above.
use crate::whatever::BlahMigrations;

/// The type that will produce a query to run when applying this migration.
#[derive(Migration)]
pub struct CreateBlahTable(Option<String>);

impl CreateBlahTable {
    fn user_column(&self) -> String {
        self.0
            .as_deref()
            .map(|d| format!("user_{d} text NOT NULL,"))
            .unwrap_or("user_default text NOT NULL".into())
    }
}

impl ResolveQuery for CreateBlahTable {
    type Ctx = BlahMigrations;

    async fn init(ctx: &mut Self::Ctx) -> TernResult<Self> {
        // This method can make use of `Self::Ctx`--whatever it is--to construct
        // the value that will become the migration query.
        let inner = ctx
            .special_value
            .clone();
        // Minor sanitizing:
        if !inner.is_none_or(|v| v.chars().all(|c| c.is_alphabetic())) {
            return Err(TernError::Invalid(
                "contains invalid characters".into(),
            ));
        }
        Ok(Self(inner))
    }

    async fn resolve(&self, _ctx: &mut Self::Ctx) -> TernResult<Query> {
        // This method can make use of `Self::Ctx` and `self` to construct the
        // query to apply.
        let user_column = self.user_column();
        let mut builder = Query::builder();

        let sql = format!("
            CREATE TABLE blah (id SERIAL PRIMARY KEY, {user_column});
        ");
        builder.push_sql(sql);

        // This builder is consumed and turned into a type that can have SQL
        // statements read from it.  This is read into a `Query`:
        let query = builder
            .build()
            .read_query()?;

        Ok(query)
    }
}
```

The macro provides metadata; the user's `ResolveQuery` provides the instructions for building the
query that will be applied.

#### Reverting migrations

Previous versions of `tern` did not support reverting migrations with a migration.  In the author's
personal experience, having a down migration available never once was useful, and in fact on several
occasions had the opposite effect.  It's easy to think that there is less risk when you have a down
migration you can always run if things go horribly wrong.

But consider: when would a down migration run?  It would run when something unexpected happened
during deployment of the up version, where the database was left in a state that is bad enough to
want to roll things back.  In this case, how safe is that "down" migration?  After all, it was
written with the idea of only how "up" could _successfully_ have been applied.  We couldn't possibly
have accounted for what led to the unexpected state for the mere fact that it was not expected.  So
how can we really have any more confidence in the down migration than we do in its associated up
migration?

With that out of the way, `tern` supports up/down migrations now.  The following section describes
how `tern` can help in building better assurance around migration sets in general, and up/down sets
more particularly.

### Testing

With the `testing` feature enabled, `tern` exposes tooling for writing tests of migrations and
running them in their own isolated database.

Add the feature in `dev-dependencies`:

```toml
[dev-dependencies]
tern = { version = "4.0.0-rc1", features = ["sqlx_postgres", "testing"] }
```

With this enabled, the proc macros `test` and `test_suite` can be used.  The `test_suite` macro
generates a test of `apply_all` that applies each migration in order:

```rust
#[derive(TernTest)]
#[tern(env = "PG_DATABASE_URL", context = BlahMigration::new)]
pub struct BlahMigrationTest {
    app: BlahMigrations,

}

impl BlahMigrationTest {}
```


#### `Property` tests

For migration sets that have pairs of up/down migrations, it additionally generates one test per pair
that exercises the up-then-down operation.  What constitutes a successful revert is only something
the user can know.  But the user can define this with a `Property` and `tern` provides the interface
to test any `Property`.  A `Property` is a set of methods:
  - `before` for something that holds before the up migration, and must still hold after the up
    migration was reverted by its down migration.
  - `after` for something that must hold after the down migration.
  - `should_revert` optionally provides a condition under which the revert should be attempted.

An example of writing properties of an up/down migration:

```rust,ignore
use tern::testing::{Properties, property_fn, TestError};

fn my_properties() -> Properties<MyApp> {
    Properties::new().with(
        1,
        property_fn(
            async |app: &mut BlahMigrations| table_absent(app, "users").await,
            async |app: &mut BlahMigrations| table_present(app, "users").await,
        ),
    )
    .with(
        2,
        property_fn(
            async |app: &mut BlahMigrations| table_absent(app, "users").await,
            async |app: &mut BlahMigrations| {
                table_present(app, "users").await?;
                if count(app, "users")
                    .await
                    .is_err_or(|n| n != 0)
                {
                    return Err(TestError::new(2, "non-zero count"))?;
                }
                Ok(())
            },
        )
        .revert_if(
            async |app: &mut BlahMigrations| {
                let n = count(app, "users").await?;
                Ok(n > 0)
            }
        )
    )
}
```

The following `test_suite` usage will generate one test for each of these
properties:

```rust
tern::test_suite! {
    app = BlahMigrations,
    source = "src/migrations",
    context = BlahMigrations::new_app,
    properties = my_properties(),
}
```

### Notes

Some miscellaneous notes:

#### Logging

`tern` provides a [`log`](https://docs.rs/log/0.4.30/log/) facade and emits logs at various levels
and at various times.  For debugging or other purpose, initialize a choice of logging implementation
before the `tern` app runs to collect these logs.

Right now there's no [`tracing`](https://docs.rs/tracing/latest/tracing/) support offered, but please
open a feature request if this would be useful.

#### Compilation targets

Because migrations are part of the source code, and .sql files are not generally expected to be
source code, it is possible that changes to a .sql file will not trigger a recompilation of the
target.  In most cases it _should_ by some macro business, but the author currently doesn't know why
this is not true on occasion.

A `cargo clean -p the-package` fixes it, but to resolve once and for all, a `build.rs` can be placed
in the root of the crate with these contents:

```rust
fn main() {
    println!("cargo:rerun-if-changed=src/migrations");
}
```

## Contributing

Contributions in the form of PR, feature request, or bug report are all very much appreciated.
Currently, a decent place to contribute in the feature department could be to add integrations with
more third-party database crates.  Or it could not--this author only knows `sqlx` and can't say much
about the popularity of other options.

Enhancements, defects, or general issues of behavior (flaws, if you will) belong in an issue with
that label.

## Licence

This project is licensed under either of:
* MIT license ([LICENSE-MIT](./LICENSE-MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE)).

[tern-cratesio]: https://crates.io/crates/tern
[tern-docsrs]: https://docs.rs/tern/latest/tern/
[eg]: ./examples
[`3.1.x`]: https://github.com/quasi-coherent/tern/tree/v3.1.x
[tern-wiki]: https://en.wikipedia.org/wiki/Elegant_tern
[sqlx-conn]: https://docs.rs/sqlx/0.8.6/sqlx/trait.Connection.html
[clap]: https://docs.rs/clap/latest/clap/
