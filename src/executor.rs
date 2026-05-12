//! Database clients for migration operations.
//!
//! An `Executor` collects all of the queries that could be sent in the course
//! of executing some migration operation.  It is required for a context to own
//! an `Executor`.
//!
//! These queries are implemented for third-party database client crates and
//! the `Executor` for the client is exposed here.
pub use tern_core::executor::{Executor, HistoryTable};

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
