use sqlx::sqlite::Sqlite;
use tern_core::context::HistoryRelid;
use tern_core::migration::MigrationData;

use crate::backend::ExecutorBackend;
use crate::backend::sqlite::SqliteBackend;
use crate::sqlx_executor::{SqlxAnyExecutor, SqlxAnyExecutorOptions};

/// `MigrationExecutor` for a Sqlite backend.
pub type SqlxSqliteExecutor = SqlxAnyExecutor<Sqlite>;

/// `ExecutorOptions` for a SQLite backend.
pub type SqlxSqliteExecutorOptions = SqlxAnyExecutorOptions<Sqlite>;

impl ExecutorBackend for Sqlite {
    fn check_history(history: HistoryRelid) -> String {
        SqliteBackend::check_history(history)
    }

    fn init_history_query(history: HistoryRelid) -> String {
        SqliteBackend::init_history_query(history)
    }

    fn drop_history_query(history: HistoryRelid) -> String {
        SqliteBackend::drop_history_query(history)
    }

    fn get_applied_where_query(
        history: HistoryRelid,
        _: Option<i64>,
        _: Option<i64>,
    ) -> String {
        // The bound values in sqlx::query are i64, not Option<i64>.
        format!(
            "
SELECT
  version,
  description,
  content,
  duration_ms,
  applied_at
FROM
  {history}
WHERE
  version >= ?1
  AND version <= ?2
ORDER BY
  version;
"
        )
    }

    fn insert_applied_query(
        history: HistoryRelid,
        _: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES (?1, ?2, ?3, ?4, ?5);
"
        )
    }

    fn delete_applied_query(history: HistoryRelid, _: i64) -> String {
        format!(
            "
DELETE FROM {history}
WHERE version = ?1;
"
        )
    }

    fn upsert_applied_query(
        history: HistoryRelid,
        _: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES (?1, ?2, ?3, ?4, ?5)
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
