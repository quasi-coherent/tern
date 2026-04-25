//! A [`MigrationExecutor`](tern_core::executor::MigrationExecutor) for
//! [`sqlx::SqlitePool`].
use sqlx::Sqlite;
use tern_core::executor::HistoryTable;

use super::{SqlxExecutor, SqlxQueryLib};

/// `SqlxSqliteExecutor` is a [`MigrationExecutor`][1] via
/// [`SqlxExecutor`](super::SqlxExecutor).
///
/// [1]: tern_core::executor::MigrationExecutor
pub type SqlxSqliteExecutor = SqlxExecutor<Sqlite>;

impl SqlxQueryLib for Sqlite {
    fn check_history(history: HistoryTable) -> String {
        let tablename = history.tablename();
        format!(
            "
SELECT EXISTS (
  SELECT 1
  FROM sqlite_master
  WHERE type = 'table'
  AND name = '{tablename}'
);
"
        )
    }

    fn create_history_if_not_exists_query(history: HistoryTable) -> String {
        let tablename = history.tablename();

        format!(
            "
CREATE TABLE IF NOT EXISTS {tablename}(
  version bigint PRIMARY KEY,
  description text NOT NULL,
  content text NOT NULL,
  duration_ms bigint NOT NULL,
  applied_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
);
"
        )
    }

    fn drop_history_query(history: HistoryTable) -> String {
        let tablename = history.tablename();
        format!("DROP TABLE IF EXISTS {tablename};")
    }

    fn get_all_applied_query(history: HistoryTable) -> String {
        let tablename = history.tablename();
        format!(
            "
SELECT
  version,
  description,
  content,
  duration_ms,
  applied_at
FROM
  {tablename}
ORDER BY
  version;
"
        )
    }

    fn insert_applied_query(history: HistoryTable) -> String {
        let tablename = history.tablename();
        format!("
INSERT INTO {tablename}(version, description, content, duration_ms, applied_at)
  VALUES (?1, ?2, ?3, ?4, ?5);
"
        )
    }

    fn delete_applied_query(history: HistoryTable, version: i64) -> String {
        let tablename = history.tablename();
        format!(
            "
DELETE FROM {tablename}
WHERE version > {version};
"
        )
    }

    fn upsert_applied_query(history: HistoryTable) -> String {
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
