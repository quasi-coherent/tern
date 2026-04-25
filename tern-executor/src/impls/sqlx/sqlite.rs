use sqlx::sqlite::Sqlite;
use tern_core::executor::HistoryTable;
use tern_core::migration::Applied;

use super::any::SqlxExecutor;
use crate::query::ExecQueryLib;
use crate::query::sqlite::SqliteExecQuery;

pub use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

/// `Executor` for a Sqlite backend.
pub type SqlxSqliteExecutor = SqlxExecutor<Sqlite>;

impl ExecQueryLib for Sqlite {
    fn check_history(history: HistoryTable) -> String {
        SqliteExecQuery::check_history(history)
    }

    fn create_history_if_not_exists_query(history: HistoryTable) -> String {
        SqliteExecQuery::create_history_if_not_exists_query(history)
    }

    fn drop_history_query(history: HistoryTable) -> String {
        SqliteExecQuery::drop_history_query(history)
    }

    fn get_all_applied_query(history: HistoryTable) -> String {
        SqliteExecQuery::get_all_applied_query(history)
    }

    fn insert_applied_query(history: HistoryTable, _: &Applied) -> String {
        let tablename = history.tablename();
        format!(
            "
INSERT INTO {tablename}(version, description, content, duration_ms, applied_at)
  VALUES (?1, ?2, ?3, ?4, ?5);
"
        )
    }

    fn delete_applied_query(history: HistoryTable, _: i64) -> String {
        let history_table = history.full_name();
        format!(
            "
DELETE FROM {history_table}
WHERE version = ?1;
"
        )
    }

    fn upsert_applied_query(history: HistoryTable, _: &Applied) -> String {
        let tablename = history.tablename();
        format!(
            "
INSERT INTO {tablename}(version, description, content, duration_ms, applied_at)
  VALUES (?1, ?2, ?3, ?4, ?5)
  ON CONFLICT REPLACE;
"
        )
    }
}
