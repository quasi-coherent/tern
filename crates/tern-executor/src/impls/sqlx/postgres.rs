use sqlx::postgres::Postgres;
use tern_core::executor::HistoryTable;
use tern_core::migration::Applied;

use super::any::SqlxExecutor;
use crate::query::ExecQueryLib;
use crate::query::postgres::PgExecQuery;

pub use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};

/// `Executor` for a PostgreSQL backend.
pub type SqlxPgExecutor = SqlxExecutor<Postgres>;

impl ExecQueryLib for Postgres {
    fn check_history(history: HistoryTable) -> String {
        PgExecQuery::check_history(history)
    }

    fn create_history_if_not_exists_query(history: HistoryTable) -> String {
        PgExecQuery::create_history_if_not_exists_query(history)
    }

    fn drop_history_query(history: HistoryTable) -> String {
        PgExecQuery::drop_history_query(history)
    }

    fn get_all_applied_query(history: HistoryTable) -> String {
        PgExecQuery::get_all_applied_query(history)
    }

    fn insert_applied_query(history: HistoryTable, _: &Applied) -> String {
        let history_table = history.full_name();
        format!("
INSERT INTO {history_table}(version, description, content, duration_ms, applied_at)
  VALUES ($1, $2, $3, $4, $5);
")
    }

    fn delete_applied_query(history: HistoryTable, _: i64) -> String {
        let history_table = history.full_name();
        format!(
            "
DELETE FROM {history_table}
WHERE version = $1;
"
        )
    }

    fn upsert_applied_query(history: HistoryTable, _: &Applied) -> String {
        let history_table = history.full_name();
        format!(
            "
INSERT INTO {history_table}(version, description, content, duration_ms, applied_at)
  VALUES ($1, $2, $3, $4, $5)
  ON CONFLICT (version) DO UPDATE
  SET
    description = excluded.description,
    content = excluded.content,
    duration_ms = excluded.duration_ms,
    applied_at = excluded.applied_at;
"
        )
    }
}
