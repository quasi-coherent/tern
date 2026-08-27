//! Migration executors.
//!
//! This module contains the [`MigrationExecutor`] interface and also provides
//! implementations of it for select third-party database client crates.
pub use tern_core::context::{ConnStr, MigrationExecutor};

#[cfg(feature = "cli")]
pub use tern_cli::ConnOpt;

#[cfg(feature = "_sqlx")]
pub use tern_executor::sqlx_executor::SqlxError;
#[cfg(feature = "_sqlx")]
pub extern crate sqlx;

#[cfg(feature = "sqlx_mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_mysql")))]
#[doc(inline)]
pub use tern_executor::sqlx_executor::{
    SqlxMySqlExecutor, SqlxMySqlExecutorOptions,
};

#[cfg(feature = "sqlx_postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_postgres")))]
#[doc(inline)]
pub use tern_executor::sqlx_executor::{SqlxPgExecutor, SqlxPgExecutorOptions};

#[cfg(feature = "sqlx_sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_sqlite")))]
#[doc(inline)]
pub use tern_executor::sqlx_executor::{
    SqlxSqliteExecutor, SqlxSqliteExecutorOptions,
};
