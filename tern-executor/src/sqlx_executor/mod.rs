//! Implementation of `MigrationExecutor` for the [`sqlx`] connection pool type.
//!
//! [`sqlx`]: sqlx::Pool
use tern_core::executor::HistoryTable;
use tern_core::error::{MigrationError, ErrorKind};

mod pool;
#[doc(inline)]
pub use pool::SqlxExecutor;

#[cfg(feature = "sqlx_postgres")]
mod postgres;
#[cfg(feature = "sqlx_postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_postgres")))]
#[doc(inline)]
pub use postgres::SqlxPgExecutor;

#[cfg(feature = "sqlx_mysql")]
mod mysql;
#[cfg(feature = "sqlx_mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_mysql")))]
#[doc(inline)]
pub use mysql::SqlxMySqlExecutor;

#[cfg(feature = "sqlx_sqlite")]
mod sqlite;
#[cfg(feature = "sqlx_sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_sqlite")))]
#[doc(inline)]
pub use sqlite::SqlxSqliteExecutor;

/// Helper trait to provide db-specific query SQL.
trait SqlxQueryLib {
    /// Query for `MigrationExecutor::check_history`.
    fn check_history(history: HistoryTable) -> String;

    /// Query for `MigrationExecutor::create_history_if_not_exists`.
    fn create_history_if_not_exists_query(history: HistoryTable) -> String;

    /// Query for `MigrationExecutor::drop_history`.
    fn drop_history_query(history: HistoryTable) -> String;

    /// Query for `MigrationExecutor::get_all_applied`.
    fn get_all_applied_query(history: HistoryTable) -> String;

    /// Query for `MigrationExecutor::insert_applied`.
    fn insert_applied_query(history: HistoryTable) -> String;

    /// Query for `MigrationExecutor::reset_last_applied`.
    fn reset_last_applied_query(history: HistoryTable, version: i64) -> String;

    /// Query for `MigrationExecutor::upsert_applied`.
    fn upsert_applied_query(history: HistoryTable) -> String;
}

impl MigrationError for sqlx::Error {
    fn message(&self) -> String {
        self.to_string()
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Executor
    }
}
