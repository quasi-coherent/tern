use tern_core::context::HistoryRelid;
use tern_core::migration::MigrationData;

use super::MySqlBackend;
use crate::backend::ExecutorBackend;

impl ExecutorBackend for MySqlBackend {
    fn check_history(history: HistoryRelid) -> String {
        let table = history.relname();
        let schema = match history.relschema() {
            Some(ns) => format!("table_schema = '{ns}'"),
            _ => "table_schema = DATABASE()".into(),
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
) AS counted;
"
        )
    }

    fn init_history_query(history: HistoryRelid) -> String {
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

    fn drop_history_query(history: HistoryRelid) -> String {
        format!("DROP TABLE IF EXISTS {history};")
    }

    fn get_applied_where_query(
        history: HistoryRelid,
        min_version: Option<i64>,
        max_version: Option<i64>,
    ) -> String {
        let minv = match min_version {
            Some(v) => format!("version >= {v}"),
            _ => "true".into(),
        };
        let maxv = match max_version {
            Some(v) => format!("version <= {v}"),
            _ => "true".into(),
        };
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
  {minv}
  AND {maxv}
ORDER BY
  version;
"
        )
    }

    fn insert_applied_query(
        history: HistoryRelid,
        data: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES ({}, '{}', '{}', {}, '{}');
",
            data.version(),
            data.description(),
            data.content(),
            data.duration_millis(),
            data.applied_at(),
        )
    }

    fn delete_applied_query(history: HistoryRelid, version: i64) -> String {
        format!(
            "
DELETE FROM {history}
WHERE version = {version};
"
        )
    }

    fn upsert_applied_query(
        history: HistoryRelid,
        data: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES ({}, '{}', '{}', {}, '{}')
  ON DUPLICATE KEY
  UPDATE
    description = VALUES(description),
    content = VALUES(content),
    duration_ms = VALUES(duration_ms),
    applied_at = VALUES(applied_at);
",
            data.version(),
            data.description(),
            data.content(),
            data.duration_millis(),
            data.applied_at(),
        )
    }
}
