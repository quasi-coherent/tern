use sqlx::mysql::MySql;
use tern_core::executor::HistoryTable;
use tern_core::migration::Applied;

use super::any::SqlxExecutor;
use crate::query::ExecQueryLib;
use crate::query::mysql::MySqlExecQuery;

pub use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions};

/// `Executor` for a MySQL backend.
pub type SqlxMySqlExecutor = SqlxExecutor<MySql>;

impl ExecQueryLib for MySql {
    fn check_history(history: HistoryTable) -> String {
        MySqlExecQuery::check_history(history)
    }

    fn create_history_if_not_exists_query(history: HistoryTable) -> String {
        MySqlExecQuery::create_history_if_not_exists_query(history)
    }

    fn drop_history_query(history: HistoryTable) -> String {
        MySqlExecQuery::drop_history_query(history)
    }

    fn get_all_applied_query(history: HistoryTable) -> String {
        MySqlExecQuery::get_all_applied_query(history)
    }

    fn insert_applied_query(history: HistoryTable, _: &Applied) -> String {
        let history_table = history.full_name();
        format!("
INSERT INTO {history_table}(version, description, content, duration_ms, applied_at)
  VALUES (?, ?, ?, ?, ?);
")
    }

    fn delete_applied_query(history: HistoryTable, _: i64) -> String {
        let history_table = history.full_name();
        format!(
            "
DELETE FROM {history_table}
WHERE version = ?;
"
        )
    }

    fn upsert_applied_query(history: HistoryTable, _: &Applied) -> String {
        let history_table = history.full_name();
        format!(
            "
INSERT INTO {history_table}(version, description, content, duration_ms, applied_at)
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
