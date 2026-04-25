use sqlx::sqlite::Sqlite;
use tern_core::migration::{HistoryTable, MigrationData};

use crate::backend::ExecutorBackend;
use crate::backend::sqlite::SqliteBackend;
use crate::sqlx_executor::{SqlxAnyExecutor, SqlxAnyExecutorOptions};

pub use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

/// `MigrationExecutor` for a Sqlite backend.
pub type SqlxSqliteExecutor = SqlxAnyExecutor<Sqlite>;

/// `ExecutorOptions` for a SQLite backend.
pub type SqlxSqliteExecutorOptions = SqlxAnyExecutorOptions<Sqlite>;

impl ExecutorBackend for Sqlite {
    fn check_history(history: HistoryTable) -> String {
        SqliteBackend::check_history(history)
    }

    fn init_history_query(history: HistoryTable) -> String {
        SqliteBackend::init_history_query(history)
    }

    fn drop_history_query(history: HistoryTable) -> String {
        SqliteBackend::drop_history_query(history)
    }

    fn get_all_applied_query(history: HistoryTable) -> String {
        SqliteBackend::get_all_applied_query(history)
    }

    fn insert_applied_query(
        history: HistoryTable,
        _: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES (?1, ?2, ?3, ?4, ?5);
"
        )
    }

    fn delete_applied_query(history: HistoryTable, _: i64) -> String {
        format!(
            "
DELETE FROM {history}
WHERE version = ?1;
"
        )
    }

    fn upsert_applied_query(
        history: HistoryTable,
        _: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES (?1, ?2, ?3, ?4, ?5)
  ON CONFLICT REPLACE;
"
        )
    }
}
