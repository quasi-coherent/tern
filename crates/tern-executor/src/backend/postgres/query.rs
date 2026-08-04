use tern_core::context::RelationId;
use tern_core::migration::MigrationData;

use super::PgBackend;
use crate::backend::ExecutorBackend;

impl ExecutorBackend for PgBackend {
    fn check_history(history: RelationId) -> String {
        let table = history.relname();
        let schema = match history.relschema() {
            Some(ns) => format!("schemaname = '{ns}'"),
            _ => "schemaname = current_schema()".into(),
        };

        format!(
            "
SELECT EXISTS (
  SELECT 1 FROM pg_tables
  WHERE {schema}
  AND tablename = '{table}'
);
"
        )
    }

    fn init_history_query(history: RelationId) -> String {
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

    fn drop_history_query(history: RelationId) -> String {
        format!("DROP TABLE IF EXISTS {history};")
    }

    fn get_applied_where_query(
        history: RelationId,
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
        history: RelationId,
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

    fn delete_applied_query(history: RelationId, version: i64) -> String {
        format!(
            "
DELETE FROM {history}
WHERE version = {version};
"
        )
    }

    fn upsert_applied_query(
        history: RelationId,
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
