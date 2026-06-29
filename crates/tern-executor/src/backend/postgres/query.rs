use tern_core::migration::{HistoryTable, MigrationData};

use super::PgBackend;
use crate::backend::ExecutorBackend;

impl ExecutorBackend for PgBackend {
    fn check_history(history: HistoryTable) -> String {
        let table = history.table();
        let schema = match history.schema() {
            Some(ns) => format!("schemaname = '{ns}'"),
            _ => "true".into(),
        };

        format!(
            "
SELECT EXISTS (
  SELECT 1 FROM pg_tables
  WHERE {schema}
  AND table = '{table}'
);
"
        )
    }

    fn init_history_query(history: HistoryTable) -> String {
        format!(
            "
CREATE TABLE IF NOT EXISTS {history}(
  version bigint PRIMARY KEY,
  description text NOT NULL,
  content text NOT NULL,
  duration_ms bigint NOT NULL,
  applied_at timestamptz NOT NULL DEFAULT now()
);"
        )
    }

    fn drop_history_query(history: HistoryTable) -> String {
        format!("DROP TABLE IF EXISTS {history};")
    }

    fn get_all_applied_query(history: HistoryTable) -> String {
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
  version > 0
ORDER BY
  version;
"
        )
    }

    fn insert_applied_query(
        history: HistoryTable,
        applied: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES ({}, '{}', '{}', {}, '{}');
",
            applied.version(),
            applied.description_ref(),
            applied.content_ref(),
            applied.duration_millis(),
            applied.applied_at(),
        )
    }

    fn delete_applied_query(history: HistoryTable, version: i64) -> String {
        format!(
            "
DELETE FROM {history}
WHERE version = {version};
"
        )
    }

    fn upsert_applied_query(
        history: HistoryTable,
        applied: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES ({}, '{}', '{}', {}, '{}')
  ON CONFLICT (version) DO UPDATE
  SET
    description = excluded.description,
    content = excluded.content,
    duration_ms = excluded.duration_ms,
    applied_at = excluded.applied_at;
",
            applied.version(),
            applied.description_ref(),
            applied.content_ref(),
            applied.duration_millis(),
            applied.applied_at(),
        )
    }
}
