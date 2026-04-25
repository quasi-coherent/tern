# tern

> A bilingual Rust framework for managing migrations targeting a MySQL, PostgreSQL, or SQLite backend.


[![Build status](https://github.com/quasi-coherent/tern/actions/workflows/main.yaml/badge.svg?branch=master)](https://github.com/quasi-coherent/tern/actions/workflows/main.yaml)
[![Crates.io](https://img.shields.io/crates/v/tern)](https://github.com/quasi-coherent/tern)
[![Documentation](https://docs.rs/tern/badge.svg)][tern-doc]

### High level features

- Integrate an existing application with its migrations: the app and migration source can be embedded in
  a binary target.  Derive macros make this a minimum of effort.
- Migration logic can be written in SQL or Rust.  Rust migrations get a user-defined context to build the
  query at runtime.
- A migration may be many individual statements, and any subset of them may be chosen to run together in a
  transaction or not.
- The backend for tracking migration state may be given a custom location, which allows for multiple,
  independent migration sets to exist in the same database.

For more, check out some [examples][eg] of `tern` applications.  The official Rust docs can be consulted
[here][tern-doc].

### ⚠ Breaking changes ⚠

The default branch is a major release candidate so it contains many breaking changes over the previous release.
See the [`3.1.x`] branch for the current non-experimental release of `tern`.

### A note on a noun

This project is called `tern`.  Apparently, so are many other database migration projects.

I can only assume this means that the migratory [species][tern-wiki] has a larger portion of their budget
going to SEO services than other families of birds that are known for having extremely long migratory patterns.

It's hard to find any that can measure up to the tern though.  Recent studies establish that the Arctic tern,
for instance, covers a round-trip length of 70,000km each year, which makes me wonder if they do anything but
migrate.

## Usage

To install `tern`, you select a supported third-party database crate to bring in as a dependency that matches
your target database.  Currently, there is support for the [`sqlx`][sqlx-pool] connection pool types for MySQL,
PostgreSQL, and SQLite.  Add this to your Cargo.toml, for example:

```toml
tern = { version = "4.0.0-rc1", features = ["sqlx_postgres"] }
```

A `tern` application consists of two things: a migration set, or the queries representing the version history,
and a `TernApp` for exposing methods to operate on the database with them.  Both are provided by an appropriate
use of the derive macro `TernApp`.

A "kitchen sink" example of what that looks like is:

```rust
use tern::TernApp;
use tern::executor::sqlx::SqlxPgExecutor;

/// Migrations and a context to apply them.
///
/// Here `schema` and `table` drive where the database state relative to the
/// migrations should be stored.  Both are optional.  Once the table exists,
/// this obviously should not change.
///
/// The `source` attribute is where the migrations live.
#[derive(Clone, Debug, TernApp)]
#[tern(source = "src/migrations", schema = "tern_history", table = "blah")]
pub struct BlahMigrations {
    /// We need a database client that implements a particular "utility" query
    /// interface.  The Cargo feature brought in `SqlxPgExecutor`, which does
    /// this.  The attribute `executor_via` points out the field to grab it from.
    ///
    /// This is technically optional, but without it `BlahMigrations` itself
    /// would need to provide the lower level database methods.
    #[tern(executor_via)]
    pub exec: SqlxPgExecutor,

    /// Whatever you desire.
    pub special_value: Option<String>,
}
```

This should be in the immediate parent of the migration source directory, so in this case
either `src/migrations/mod.rs` or `src/migrations.rs` depending on flavor.

`BlahMigrations` can now be turned into a runnable application:

```rust
use tern::Tern;
use tern::executor::ConnStr;
use tern::executor::sqlx::SqlxPgExecutor;
use tern::ops::{ApplyArgs, ListArgs};

// Constructing the interior "utility" client, or "executor" as it were.
// `ConnStr` can build simple versions of "executors" but more precise
// construction is possible too.
let conn = ConnStr::from_env("DATABASE_URL")?;
let exec: SqlxPgExecutor = conn.connect().await?;

// Our custom `TernApp`:
let special_value = Some("lebron_james".into());
let blah = BlahMigrations { exec, special_value };

// `Tern` wraps the app and exposes "main" methods.
let mut app = Tern::new(blah);

// `List` is an operation to show migrations that exist.
//
// `diff` returns the diff between local and remote sources, which is just the
// unapplied migrations.
let list_args = ListArgs::new().diff();

let unapplied = app.list(list_args).await?;
println!("unapplied migrations: {unapplied}");

// Looks good :+1:
//
// The `Apply` operation with `all` option runs all unapplied migrations.
let apply_args = ApplyArgs::new().all();
match app.apply(apply_args).await {
    Ok(complete) => println!("migration complete, applied: {complete}"),
    Err(e) => println!("failed migration, partial result and error: {e}"),
}
```

### CLI

With the `cli` feature enabled, these operations and arguments can be supplied
on the command line to simplify things.

```rust
use tern::Tern;
// The `clap::Args` version of `ConnStr` above.
use tern::executor::ConnOpt;

// We only provide the async closure to construct our app given a `ConnOpt`.
//
// This has the effect of insisting the CLI parser can get a `ConnOpt`.  In this
// particular case, either `--database-url` or the environment variable
// `DATABASE_URL` should be provided the connection string, and this will work.
//
// `Tern::run_with` uses the output of our closure to run the command subject to
/// the options that were parsed via CLI:
let result =
  Tern::run_with(|opts: ConnOpt| async move {
      let exec: SqlxPgExecutor = opts.connect().await?;
      let special_value = Some("lebron_james".into());
      Ok(BlahMigrations { exec, special_value })
  })
  .await;

match result {
    Ok(complete) => println!("operation success: {complete}"),
    Err(e) => println!("operation failed: {e}"),
}
```

### Migrations

Migrations are part of the Rust source code, located in the directory that the `source`
attribute of `TernApp` references, a path relative to `CARGO_MANIFEST_DIR`.

These files are expected to follow these conventions/rules:

* A migration file can be in Rust (more [below](#rust-migrations)) or in SQL.  The eventual
  output should be a prepared statement to send to the database.
* A migration query can have one or more constituent parts (expressions ending with `;`).
* Migration source filenames must match the regex pattern `^(V|U|D)(\d+)__(\w+)\.(sql|rs)$`.
  For example,
  - `V1__create_a_table.sql`
  - `V5__create_a_different_table.rs`
  - `U91__create_a_table_again.sql`
* A migration set can come in pairs, prefixed with `U` (for "up") and `D` (for "down"), when
  one up migration is ostensibly reverted by its corresponding down migration.
  For example
  - `U8__create_table_index.rs` and `D8__create_table_index.sql`
  - `U22__do_a_thing.rs` and `D22__do_a_thing.sql`
* One up/down pair can be any combination of .rs and .sql, and a migration set must have all up/down
  or no up/down pairs.

If a migration set is an up/down type, additional operations for reverting the version of the
database are enabled.  More on down migrations [below](#reverting-migrations).

#### SQL annotations

We proclaimed at the top that you can control what, if anything, runs in a database transaction
and what does not.  This becomes important in some real scenarios.  Some migration tooling will run
things in a transaction and it is not possible to override this, but for instance it is an error to
try to create an index with the `CONCURRENTLY` keyword in PostgreSQL.

To facilitate, `tern` understands certain magic keywords that influence how a migration's query is
interpreted:

* `tern:noTransaction` should be found somewhere in a comment on the first line.  When found, `tern`
  will build and run this query without opening a database transaction first.
* `tern:begin_tx` in a comment can decorate one individual statement.  This instructs `tern` to group
  this and all subsequent statements until closed into one prepared statement sent to the database.
  This has the effect of running the group of queries in a transaction.
* `tern:end_tx` closes a previous `tern:begin_tx` _after the statement it decorates_.  So in summary,
  a `tern:begin_tx` and `tern:end_tx` includes everything from the `begin` through and including the
  query under an `end` annotation.
* A SQL dialect hint can be given by `tern:postgres`.  Supported values are `mysql`, `postgres`, and
  `sqlite`.  This must be on the first line in a comment, together with `noTransaction` if both should
  be included: `tern:noTransaction,postgres`.  This will rarely be useful in the case that uncommon,
  dialect-specific syntax causes confuses the interpretation of statement terminating characters.

```sql
-- tern:noTransaction
-- The migration itself will not be surrounded by a transaction now.

-- This runs on its own.
CREATE TABLE IF NOT EXISTS whatever (
  we_id serial PRIMARY KEY,
  created_at timestamptz(3) NOT NULL DEFAULT now(),
  user_id uuid NOT NULL DEFAULT uuid_generate_v1mc(),
);

-- tern:begin_tx
-- Everything below this is included in one statement.
CREATE TABLE IF NOT EXISTS whatever_whatever (
  wewe_id serial PRIMARY KEY,
  we_created_at timestamptz(3) REFERENCES whatever(we_id),
  last_login_at timestamptz(3),
  user_name text,
);

-- This means the group ends after this index creation:
-- tern:end_tx
CREATE INDEX IF NOT EXISTS wewe_user_idx
  ON whatever_whatever (we_created_at);

-- This runs on its own:
SELECT 'comme ci, comme ca';
```

#### Rust migrations

The migration source files become bona fide modules in the Rust source tree, and each is unified by
the [`Migration`](https://docs.rs/tern/latest/tern/trait.Migration.html) interface.  This is done
automatically for SQL sources, but not quite for Rust sources.

The derive macro for `Migration` will do it, but only with a little help from the user to write out
something that would need to be written out in any case:

```rust
use tern::{Migration, Query, TernResult, ResolveMigration};

// This is the type deriving `TernApp` from above.
// It's assumed the `TernApp` exists in the parent of this and all migrations.
use super::BlahMigrations;

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

impl ResolveMigration for CreateBlahTable {
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
        // statements read from it.  This is "read_into" a `Query`:
        let query = builder
            .into_reader()
            .read_into()?;

        Ok(query)
    }
}
```

The macro provides metadata; the user's `ResolveMigration` provides the instructions for building
the query that will be applied.

#### Reverting migrations

Previous versions of `tern` did not support reverting migrations.  Down migrations have never been
used for a useful purpose in the author's experience, and the opinion is that instead they are mostly
hurtful for providing a false confidence during deployment and often making a bad situation worse.

Think about when a down migration would run: it would run when something unexpected happened during
deployment of the up version and the database was left in a state that is bad enough to revert.  So,
given this, how much faith should we have in a "down" migration that was written with only the
understanding of how "up" could _successfully_ be applied?  We can't possibly have accounted for what
led to the unexpected state for the mere fact that it was not expected.

Having gotten that down, `tern` now supports up/down migrations.  An important note is that a down
migration will _always_ be applied outside of a database transaction, and each statement in the file
is ran on its own, ignoring any annotations that may be found.

The intended purpose for this is to best support carefully reverting each up migration component,
one-by-one, to ensure that the database is not left in an unknown state in case a failure occurs
midway.  If an error is encountered, the statement's (0-based) index within the migration file can be
given as an argument to the revert operation on a retry in order to resume from the point of failure.

### Notes

Some miscellaneous notes:

#### Logging

`tern` provides a [`log`](https://docs.rs/log/0.4.30/log/) facade and emits logs at various levels
and at various times.  For debugging or other purpose, initialize a choice of logging implementation
before the `tern` app runs to collect these logs.

Right now there's no `tracing` support offered, but please open an issue if this would be useful.

#### Compilation targets

Because migrations are part of the source code, and .sql files are not generally expected to fit that
definition, it is possible that changes to a .sql file will not trigger a recompilation of the target.
In most cases it _should_ by some macro business, but the author currently doesn't know why this fails
to be true on occasion.

A `cargo clean -p the-package` fixes it, but to resolve once and for all, a `build.rs` can be placed
in the root of the crate with these contents:

```rust
fn main() {
    println!("cargo:rerun-if-changed=src/migrations");
}
```

## Contributing

Contributions in the form of PR, feature request, or bug report are all very much appreciated.
Currently, a decent place to contribute in the feature department could be to add integrations
with more third-party database crates.  Or it could not--this author only knows `sqlx` and can't
say much about the popularity of other options.

Enhancements, defects, or general issues of behavior (flaws, if you will) belong in an issue with
that label.

## Licence

This project is licensed under either of:
* MIT license ([LICENSE-MIT](./LICENSE-MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE)).

[tern-doc]: https://docs.rs/tern/latest/tern/
[eg]: ./examples
[`3.1.x`]: https://github.com/quasi-coherent/tern/tree/v3.1.x
[tern-wiki]: https://en.wikipedia.org/wiki/Elegant_tern
[sqlx-pool]: https://docs.rs/sqlx/0.9.0/sqlx/struct.Pool.html
[clap]: https://docs.rs/clap/latest/clap/
