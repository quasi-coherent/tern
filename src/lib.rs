//! # tern
//!
//! A database migration library and CLI supporting embedded migrations written
//! in SQL or Rust.
//!
//! It aims to support static SQL migration sets, but expands to work with
//! migration queries written in Rust that are either statically determined or
//! that need to be dynamically built at the time of being applied.  It also aims
//! to do this while being agnostic to the particular choice of crate for
//! database interaction.
//!
//! ## Executors
//!
//! The abstract [`Executor`] is the type responsible for actually connecting to
//! a database and issuing queries.  Right now, this project supports all of the
//! [`sqlx`][sqlx-repo] pool types via the generic [`Pool`][sqlx-pool], so that
//! includes PostgreSQL, MySQL, and SQLite. These can be enabled via feature
//! flag.
//!
//! Adding more executors is welcomed! That can be in PR form or as a feature
//! request.  Adding an executor seems like it should not be hard.
//!
//! ## Usage
//!
//! Embedded migrations are prepared, built, and ran off a directory living in
//! a Rust project's source. These stages are handled by three separate traits,
//! but implementing any of them is generally not necessary.  `tern` exposes
//! derive macros that supply everything needed:
//!
//! * [`MigrationSource`]: Given the required `source` macro attribute, which is
//!   a path to the directory containing the migrations, it prepares the
//!   migration set that is required of the given operation requested.
//! * [`MigrationContext`]: Generates what is needed of the context to be an
//!   acceptable type used in the [`Runner`].  It has the field attribute
//!   `executor_via` that can decorate a field of the struct that has some
//!   [`Executor`], or connection type.  The context can build migration queries
//!   and run them.
//!
//! Put together, that looks like this.
//!
//! ```rust,no_run
//! # async fn asdf() -> String {
//! use tern::executor::SqlxPgExecutor;
//! use tern::{MigrationSource, MigrationContext, Runner};
//!
//! /// `$CARGO_MANIFEST_DIR/src/migrations` is a collection of migration files.
//! /// The optional `table` attribute permits a custom location for a migration
//! /// history table.
//! #[derive(MigrationSource, MigrationContext)]
//! #[tern(source = "src/migrations", table = "example")]
//! struct Example {
//!    // `Example` itself needs to be an executor without the annotation.
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
//! For a more in-depth example, see the [examples][examples-repo].
//!
//! ### Rust migrations
//!
//! Migrations can be expressed in Rust, and these can take advantage of the
//! arbitrary migration context to flexibly build the query at runtime.  To do
//! this, the context needs to know how to build the query, and what migration
//! to build it for.  This is achieved using some convention, a hand-written
//! trait implementation, and another macro:
//!
//! ```rust,no_run
//! # async fn qwer() -> i32 {
//! use tern::error::TernResult;
//! use tern::migration::{Query, QueryBuilder};
//! use tern::Migration;
//!
//! /// This is the convention: it needs to be called this for the macro
//! /// implementation.  This macro has an attribute `no_transaction` that
//! /// instructs the context associated to it by `QueryBuilder` below to run
//! /// the query outside of a transaction.
//! #[derive(Migration)]
//! pub struct TernMigration;
//!
//! impl QueryBuilder for TernMigration {
//!     /// The context from above.
//!     type Ctx = Example;
//!
//!     async fn build(&self, ctx: &mut Self::Ctx) -> TernResult<Query> {
//!         // Really anything can happen here.  It just depends on what
//!         // `Self::Ctx` can do.
//!         let sql = "SELECT 1;";
//!         let query = Query::new(sql);
//!         Ok(query)
//!     }
//! }
//!
//! # 25
//! # }
//! ```
//!
//! ## CLI
//!
//! With the feature flag "cli" enabled, this exposes a CLI that can be imported
//! into your own migration project:
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
//! > $ my-migration-project migrate --help
//! Usage: my-migration-project migrate <COMMAND>
//!
//! Commands:
//!   apply-all     Run any available unapplied migrations
//!   list-applied  List previously applied migrations
//!   new           Create a new migration with an auto-selected version and the given description
//!   help          Print this message or the help of the given subcommand(s)
//!
//! Options:
//!   -h, --help  Print help
//! ```
//!
//! [`MigrationSource`]: crate::migration::MigrationSource
//! [`MigrationContext`]: crate::migration::MigrationContext
//! [`Executor`]: crate::migration::Executor
//! [`Runner`]: crate::Runner
//! [examples-repo]: https://github.com/quasi-coherent/tern/tree/master/examples
//! [sqlx-repo]: https://github.com/launchbadge/sqlx
//! [sqlx-pool]: https://docs.rs/sqlx/0.8.3/sqlx/struct.Pool.html
//!

#[cfg(feature = "cli")]
#[doc(hidden)]
pub use tern_cli::{self as cli, App, ContextOptions};
pub use tern_core::runner::{MigrationResult, Report, Runner};
pub use tern_core::{error, migration};

pub mod executor {
    //! Specific backend implementations.  These are enabled via feature flags.
    #[cfg(feature = "sqlx_mysql")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlx_mysql")))]
    pub use tern_core::executor::SqlxMySqlExecutor;

    #[cfg(feature = "sqlx_postgres")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlx_postgres")))]
    pub use tern_core::executor::SqlxPgExecutor;

    #[cfg(feature = "sqlx_sqlite")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlx_sqlite")))]
    pub use tern_core::executor::SqlxSqliteExecutor;
}

pub mod types {
    pub use super::migration::{AppliedMigration, MigrationSet, Query};
}

pub mod future {
    pub use tern_core::future::{BoxFuture, Future};
}

#[doc(hidden)]
extern crate tern_derive;

#[doc(hidden)]
pub use tern_derive::{Migration, MigrationContext, MigrationSource};
