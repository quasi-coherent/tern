//! # tern
//!
//! A library and framework for defining database migrations in SQL or Rust and
//! embedding them in a binary, with support for PostgreSQL, MySQL, and SQLite.
//!
//! # Overview
#![cfg_attr(docsrs, feature(docs_cfg))]
#![warn(missing_docs)]
pub mod migrate;
pub use migrate::{Tern, TernMigrate};

pub mod migration;
pub use migration::{Migration, MigrationId, ResolveQuery};

mod operation;

pub mod report;
pub use report::Report;

/// Executors for `sqlx`.
///
/// This module exports an [`Executor`] for the [`sqlx::Pool`] types.
#[cfg(any(
    feature = "sqlx_mysql",
    feature = "sqlx_postgres",
    feature = "sqlx_sqlite"
))]
pub mod sqlx {
    pub use tern_executor::{SqlxError, sqlx_lib};

    #[cfg(feature = "sqlx_mysql")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlx_mysql")))]
    #[doc(inline)]
    pub use tern_executor::{SqlxMySqlExecutor, sqlx_mysql};

    #[cfg(feature = "sqlx_postgres")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlx_postgres")))]
    #[doc(inline)]
    pub use tern_executor::{SqlxPgExecutor, sqlx_postgres};

    #[cfg(feature = "sqlx_sqlite")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlx_sqlite")))]
    #[doc(inline)]
    pub use tern_executor::{SqlxSqliteExecutor, sqlx_sqlite};
}

pub use tern_core::context::MigrationContext;
#[doc(inline)]
pub use tern_core::error::{self, TernError, TernResult};
pub use tern_core::executor::{Executor, HistoryTable};
#[doc(inline)]
pub use tern_core::query::{self, Query};

#[doc(hidden)]
pub mod private {
    pub use futures_core::future::BoxFuture;
}

#[doc(hidden)]
extern crate tern_derive;

pub use tern_derive::{Migration, TernMigrate};
