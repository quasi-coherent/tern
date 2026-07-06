use sqlx::mysql::MySql;
use tern_core::context::RelationId;
use tern_core::migration::MigrationData;

use crate::backend::ExecutorBackend;
use crate::backend::mysql::MySqlBackend;
use crate::sqlx_executor::{SqlxAnyExecutor, SqlxAnyExecutorOptions};

/// `MigrationExecutor` for a MySQL backend.
pub type SqlxMySqlExecutor = SqlxAnyExecutor<MySql>;

/// `ExecutorOptions` for a MySQL backend.
pub type SqlxMySqlExecutorOptions = SqlxAnyExecutorOptions<MySql>;

impl ExecutorBackend for MySql {
    fn check_history(history: RelationId) -> String {
        MySqlBackend::check_history(history)
    }

    fn init_history_query(history: RelationId) -> String {
        MySqlBackend::init_history_query(history)
    }

    fn drop_history_query(history: RelationId) -> String {
        MySqlBackend::drop_history_query(history)
    }

    fn get_applied_where_query(
        history: RelationId,
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
  version >= ?
  AND version <= ?
ORDER BY
  version;
"
        )
    }

    fn insert_applied_query(history: RelationId, _: &MigrationData) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES (?, ?, ?, ?, ?);
"
        )
    }

    fn delete_applied_query(history: RelationId, _: i64) -> String {
        format!(
            "
DELETE FROM {history}
WHERE version = ?;
"
        )
    }

    fn upsert_applied_query(history: RelationId, _: &MigrationData) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES (?, ?, ?, ?, ?)
  ON DUPLICATE KEY
  UPDATE
    description = VALUES(description),
    content = VALUES(content),
    duration_ms = VALUES(duration_ms),
    applied_at = VALUES(applied_at);
"
        )
    }
}
