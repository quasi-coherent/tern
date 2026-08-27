use tern_core::context::HistoryRelid;
use tern_core::migration::MigrationData;

use super::SqliteBackend;
use crate::backend::ExecutorBackend;

impl ExecutorBackend for SqliteBackend {
    fn check_history(history: HistoryRelid) -> String {
        format!(
            "
SELECT EXISTS (
  SELECT 1
  FROM sqlite_master
  WHERE type = 'table'
  AND name = '{history}'
);
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
        applied: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES ({}, '{}', '{}', {}, '{}');
",
            applied.version(),
            applied.description(),
            applied.content(),
            applied.duration_millis(),
            applied.applied_at(),
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
            applied.description(),
            applied.content(),
            applied.duration_millis(),
            applied.applied_at(),
        )
    }
}
