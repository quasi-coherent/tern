//! Database clients for migration operations.
pub use tern_core::context::MigrationExecutor;
pub use tern_executor::{ConnStr, ExecutorOptions};

#[cfg(feature = "cli")]
pub use tern_cli::ConnOpt;

/// Executors for `sqlx`.
///
/// This module exports an executor for the `sqlx` pool type.  It also exports
/// the `sqlx` crate itself under the symbol `sqlx_lib`.
#[cfg(any(
    feature = "sqlx_mysql",
    feature = "sqlx_postgres",
    feature = "sqlx_sqlite"
))]
pub mod sqlx {
    pub use tern_executor::sqlx_executor::SqlxError;

    #[cfg(feature = "sqlx_mysql")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlx_mysql")))]
    #[doc(inline)]
    pub use tern_executor::sqlx_executor::mysql::{
        SqlxMySqlExecutor, SqlxMySqlExecutorOptions,
    };

    #[cfg(feature = "sqlx_postgres")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlx_postgres")))]
    #[doc(inline)]
    pub use tern_executor::sqlx_executor::postgres::{
        SqlxPgExecutor, SqlxPgExecutorOptions,
    };

    #[cfg(feature = "sqlx_sqlite")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlx_sqlite")))]
    #[doc(inline)]
    pub use tern_executor::sqlx_executor::sqlite::{
        SqlxSqliteExecutor, SqlxSqliteExecutorOptions,
    };
}

/// Utility re-exports for specific executors.
pub mod util {
    #[cfg(any(
        feature = "sqlx_mysql",
        feature = "sqlx_postgres",
        feature = "sqlx_sqlite"
    ))]
    pub extern crate sqlx;
}
