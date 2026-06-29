use sqlx::mysql::MySql;
use tern_core::migration::{HistoryTable, MigrationData};

use crate::backend::ExecutorBackend;
use crate::backend::mysql::MySqlBackend;
use crate::sqlx_executor::{SqlxAnyExecutor, SqlxAnyExecutorOptions};

pub use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions};

/// `MigrationExecutor` for a MySQL backend.
pub type SqlxMySqlExecutor = SqlxAnyExecutor<MySql>;

/// `ExecutorOptions` for a MySQL backend.
pub type SqlxMySqlExecutorOptions = SqlxAnyExecutorOptions<MySql>;

impl ExecutorBackend for MySql {
    fn check_history(history: HistoryTable) -> String {
        MySqlBackend::check_history(history)
    }

    fn init_history_query(history: HistoryTable) -> String {
        MySqlBackend::init_history_query(history)
    }

    fn drop_history_query(history: HistoryTable) -> String {
        MySqlBackend::drop_history_query(history)
    }

    fn get_all_applied_query(history: HistoryTable) -> String {
        MySqlBackend::get_all_applied_query(history)
    }

    fn insert_applied_query(
        history: HistoryTable,
        _: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES (?, ?, ?, ?, ?);
"
        )
    }

    fn delete_applied_query(history: HistoryTable, _: i64) -> String {
        format!(
            "
DELETE FROM {history}
WHERE version = ?;
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
  VALUES (?, ?, ?, ?, ?)
  ON DUPLICATE_KEY
  UPDATE
    description = VALUES(description),
    content = VALUES(content),
    duration_ms = VALUES(duration_ms),
    applied_at = VALUES(applied_at);
"
        )
    }
}
