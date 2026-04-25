use tern_core::executor::tern_sqlx::{HistoryTable, SqlxQueryLib};

use crate::sqlx_pool::SqlxExecutor;

pub use

/// Executor for the `sqlx::Postgres` database.
pub type SqlxPgExecutor = SqlxExecutor<Postgres>;

impl SqlxQueryLib for Postgres {
    fn check_history(history: HistoryTable) -> String {
        let tablename = history.name;
        let schemaname = match history.namespace {
            Some(ns) => format!("schemaname = '{ns}'"),
            _ => "true".into(),
        };
        format!(
            "
SELECT EXISTS (
  SELECT 1 FROM pg_tables
  WHERE {schemaname}
  AND tablename = '{tablename}'
);
"
        )
    }

    fn create_history_if_not_exists_query(history: HistoryTable) -> String {
        let history_table = history.full_name();
        format!(
            "
CREATE TABLE IF NOT EXISTS {history_table}(
  version bigint PRIMARY KEY,
  description text NOT NULL,
  content text NOT NULL,
  duration_ms bigint NOT NULL,
  applied_at timestamptz NOT NULL DEFAULT now()
);
"
        )
    }

    fn drop_history_query(history: HistoryTable) -> String {
        let history_table = history.full_name();
        format!("DROP TABLE IF EXISTS {history_table};")
    }

    fn get_all_applied_query(history: HistoryTable) -> String {
        let history_table = history.full_name();
        format!(
            "
SELECT
  version,
  description,
  content,
  duration_ms,
  applied_at
FROM
  {history_table}
ORDER BY
  version;
"
        )
    }

    fn insert_applied_query(history: HistoryTable) -> String {
        let history_table = history.full_name();
        format!("
INSERT INTO {history_table}(version, description, content, duration_ms, applied_at)
  VALUES ($1, $2, $3, $4, $5);
")
    }

    fn reset_last_applied_query(history: HistoryTable, version: i64) -> String {
        let history_table = history.full_name();
        format!(
            "
DELETE FROM {history_table}
WHERE version > {version};
"
        )
    }

    fn upsert_applied_query(history: HistoryTable) -> String {
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
