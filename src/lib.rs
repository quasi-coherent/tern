#![cfg_attr(docsrs, feature(doc_cfg))]

//! A database migration library and CLI supporting embedded migrations written
//! in SQL or Rust.
//!
//! It aims to support static SQL migration sets, but expands to work with
//! migration queries written in Rust that are either statically determined or
//! that need to be dynamically built at the time of being applied, while
//! being agnostic to the particular choice of crate for database interaction.
//!
//! ## Executors
//!
//! The abstract [`Executor`] is the thing ultimately responsible for actually
//! connecting to a database and issuing queries.  Right now, this project
//! supports all of the [`sqlx`][sqlx-repo] pool types via the generic
//! [`Pool`][sqlx-pool], which includes PostgreSQL, MySQL, and SQLite. These can
//! be enabled via feature flag.
//!
//! Supporting more third-party crates is definitely desired!  If yours is not
//! available here, please feel free to contribute, either with a PR or feature
//! request.  Adding a new executor seems like it should not be hard.
//!
//! ## Usage
//!
//! Embedded migrations are prepared, built, and ran off a directory living in
//! a Rust project's source.  This directory can contain `.rs` and `.sql` files
//! having names matching the regex `^V(\d+)__(\w+)\.(sql|rs)$`, e.g.,
//! `V13__create_a_table.sql` or `V5__create_a_different_table.rs`.
//!
//! The stages of a migration are handled by a few different traits, but
//! implementing any of them manually is generally not necessary; `tern` exposes
//! derive macros that do this.
//!
//! * [`MigrationSource`]: Prepares the migrations for use in some operation by
//!   parsing the directory into a sorted, uniform collection, and exposing
//!   methods to return subsets for a given operation.
//! * [`MigrationContext`]: A type providing a context to perform the operation
//!   on the migrations provided by `MigrationSource`.
//!
//! Put together, it looks like this.
//!
//! ```rust,no_run
//! # async fn asdf() -> String {
//! use tern::{MigrationSource, MigrationContext, Runner, SqlxPgExecutor};
//!
//! /// `$CARGO_MANIFEST_DIR/src/migrations` is a collection of migration files.
//! /// The optional `table` attribute permits a custom location for a migration
//! /// history table in the target database.
//! #[derive(MigrationSource, MigrationContext)]
//! #[tern(source = "src/migrations", table = "example")]
//! struct Example {
//!    // `Example` itself needs to be an executor without this annotation.
//!    #[tern(executor_via)]
//!     executor: SqlxPgExecutor,
//! }
//!
//! let executor = SqlxPgExecutor::new("postgres://user@localhost").await.unwrap();
//! let context = Example { executor };
//! let mut runner = Runner::new(context);
//! let report: tern::Report = runner.apply_all().await.unwrap();
//! println!("{report:#?}");
//!
//! # String::from("asdf")
//! # }
//! ```
//!
//! For more in-depth examples, see the [examples][examples-repo].
//!
//! ## SQL migrations
//!
//! Since migrations are embedded in the final executable, and static SQL
//! migrations are not Rust source, any change to a SQL migration won't force
//! a recompilation.  The proc macro that parses these files will then not be
//! up-to-date, and this can cause confusing issues.  To remedy, a `build.rs`
//! file can be put in the project directory with these contents:
//!
//! ```rust,no_run
//! fn main() -> {
//!     println!("cargo:rerun-if-changed=src/migrations/")
//! }
//! ```
//!
//! ## Rust migrations
//!
//! Migrations can be expressed in Rust, and these can take advantage of the
//! arbitrary migration context to flexibly build the query at runtime.  For
//! this to work, the derive macros get us nearly there, but the user needs to
//! follow a couple rules and write an implementation of a trait to complete the
//! requirements.
//!
//! The first rule is that the type deriving `MigrationSource` be declared in
//! `super` of the migrations.  So if `source = "src/migrations"`, a perfect
//! place to put a `MigrationContext`/`MigrationSource`-deriving type is in the
//! module `src/migrations.rs`, which would have to exist in any case. For now
//! this is required because it's the easiest way to know for sure how to
//! reference the module containing the migration when expanding the syntax
//! coming from macros that need it.
//!
//! The other requirement is that there be a struct called `TernMigration` in
//! that migration source, and that it derives `Migration`.  This is also
//! required for now by an implementation detail of the macros: we need a way
//! for the `Migration` macro to share data with the `MigrationSource` macro,
//! or else not use `Migration` and parse the entire Rust source file within
//! `MigrationSource` instead, which is clearly the least appealing option.
//!
//! This `TernMigration` is what is needed to apply the migration when combined
//! with the last thing required from the user: the actual query that should be
//! ran and how it runs in the custom context.  This is represented by the
//! [`QueryBuilder`] trait:
//!
//! ```rust,no_run
//! use tern::{Query, QueryBuilder, Migration};
//! use tern::error::TernResult;
//!
//! use super::Example;
//!
//! /// Use the optional macro attribute `#[tern(no_transaction)]` to avoid
//! /// running this in a database transaction.
//! #[derive(Migration)]
//! pub struct TernMigration;
//!
//! impl QueryBuilder for TernMigration {
//!     /// The custom-defined migration context.
//!     type Ctx = Example;
//!
//!     /// When `await`ed, this should produce a valid SQL query wrapped by
//!     /// `Query`.  This is what will run against the database.
//!     async fn build(&self, ctx: &mut Self::Ctx) -> TernResult<Query> {
//!         // Really anything can happen here.  It just depends on what
//!         // `Self::Ctx` can do.
//!         let sql = "SELECT 1;";
//!         let query = Query::new(sql);
//!         Ok(query)
//!     }
//! }
//! ```
//!
//! ## Reversible migrations
//!
//! As of now, the official stance is to not support an up-down style of
//! migration set, the philosophy being that down migrations are not that useful
//! in practice. The "Important Notes" section in [this][flyway-undo] flyway
//! documentation summarizes our feelings well.
//!
//! ## Database transactions
//!
//! By default, a migration and its accompanying schema history table update are
//! ran in a database transaction.  Sometimes this is not desirable and other
//! times it is not allowed.  For instance, in postgres you cannot create an
//! index `CONCURRENTLY` in a transaction.  To give the user the option, `tern`
//! understands certain annotations and will not run that migration in a
//! database transaction if they are present.
//!
//! For a SQL migration:
//!
//! ```sql
//! -- tern:noTransaction is the annotation for SQL.  It needs to be found
//! -- somewhere on the first line of the file.
//! CREATE INDEX CONCURRENTLY IF NOT EXISTS blah ON whatever;
//! ```
//!
//! For a Rust migration:
//!
//! ```rust,no_run
//! use tern::Migration;
//!
//! /// Don't run this in a migration.
//! #[derive(Migration)]
//! #[tern(no_transaction)]
//! pub struct TernMigration;
//! ```
//!
//! ## CLI
//!
//! With the feature flag "cli" enabled the type [`App`] is exported, which is a
//! CLI wrapping `Runner` methods that can be imported into your own migration
//! project to turn it into a CLI.
//!
//! ```terminal
//! > $ my-migration-project --help
//! Usage: my-migration-project <COMMAND>
//!
//! Commands:
//!   migrate  Operations on the set of migration files
//!   history  Operations on the table storing the history of these migrations
//!   help     Print this message or the help of the given subcommand(s)
//!
//! Options:
//!   -h, --help  Print help
//! ```
//!
//! [`MigrationSource`]: crate::tern_derive::MigrationSource
//! [`MigrationContext`]: crate::tern_derive::MigrationContext
//! [`Migration`]: crate::tern_derive::Migration
//! [`Executor`]: crate::Executor
//! [`Runner`]: crate::Runner
//! [examples-repo]: https://github.com/quasi-coherent/tern/tree/master/examples
//! [sqlx-repo]: https://github.com/launchbadge/sqlx
//! [sqlx-pool]: https://docs.rs/sqlx/0.8.3/sqlx/struct.Pool.html
//! [`QueryBuilder`]: crate::QueryBuilder
//! [flyway-undo]: https://documentation.red-gate.com/fd/migrations-184127470.html#Migrations-UndoMigrations
//! [`App`]: crate::App

#[doc(inline)]
pub use tern_core::error::{self, DatabaseError, Error, TernResult};

#[doc(inline)]
pub use tern_core::migration::{
    self, Executor, Migration, MigrationContext, MigrationSet, MigrationSource, Query, QueryBuilder,
};

#[doc(inline)]
pub use tern_core::runner::{self, MigrationResult, Report, Runner};

#[cfg(feature = "sqlx_mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_mysql")))]
#[doc(inline)]
pub use tern_core::executor::sqlx_backend::mysql::SqlxMySqlExecutor;

#[cfg(feature = "sqlx_postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_postgres")))]
#[doc(inline)]
pub use tern_core::executor::sqlx_backend::postgres::SqlxPgExecutor;

#[cfg(feature = "sqlx_sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_sqlite")))]
#[doc(inline)]
pub use tern_core::executor::sqlx_backend::sqlite::SqlxSqliteExecutor;

pub mod future {
    //! `futures` re-exports.
    pub use tern_core::future::{BoxFuture, Future};
}

#[cfg(feature = "cli")]
#[cfg_attr(docsrs, doc(cfg(feature = "cli")))]
#[doc(inline)]
pub use tern_cli::{App, ContextOptions};

#[doc(hidden)]
extern crate tern_derive;

pub use tern_derive::{Migration, MigrationContext, MigrationSource};
