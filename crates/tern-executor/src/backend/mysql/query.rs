use tern_core::migration::{HistoryTable, MigrationData};

use super::MySqlBackend;
use crate::backend::ExecutorBackend;

impl ExecutorBackend for MySqlBackend {
    fn check_history(history: HistoryTable) -> String {
        let table = history.table();
        let schema = match history.schema() {
            Some(ns) => format!("table_schema = '{ns}'"),
            _ => "true".into(),
        };

        format!(
            "
-- Need to return a bool like the others.
SELECT cnt = 1
FROM (
  SELECT count(*) AS cnt
  FROM information_schema.tables
  WHERE
    {schema}
    AND table_name = '{table}'
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
  applied_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
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
        data: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES ({}, '{}', '{}', {}, '{}');
",
            data.version(),
            data.description_ref(),
            data.content_ref(),
            data.duration_millis(),
            data.applied_at(),
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
        data: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES ({}, '{}', '{}', {}, '{}')
  ON DUPLICATE_KEY
  UPDATE
    description = VALUES(description),
    content = VALUES(content),
    duration_ms = VALUES(duration_ms),
    applied_at = VALUES(applied_at);
",
            data.version(),
            data.description_ref(),
            data.content_ref(),
            data.duration_millis(),
            data.applied_at(),
        )
    }
}
