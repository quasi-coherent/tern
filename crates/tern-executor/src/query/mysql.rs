use tern_core::executor::HistoryTable;
use tern_core::migration::Applied;

use super::ExecQueryLib;

/// For a MySql backend.
#[allow(unused)]
pub struct MySqlExecQuery;

impl ExecQueryLib for MySqlExecQuery {
    fn check_history(history: HistoryTable) -> String {
        let tablename = history.tablename();
        let schemaname = match history.namespace() {
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
    {schemaname}
    AND table_name = '{tablename}'
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
  applied_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
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

    fn insert_applied_query(
        history: HistoryTable,
        applied: &Applied,
    ) -> String {
        let history_table = history.full_name();
        format!("
INSERT INTO {history_table}(version, description, content, duration_ms, applied_at)
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
        let history_table = history.full_name();
        format!(
            "
DELETE FROM {history_table}
WHERE version = {version};
"
        )
    }

    fn upsert_applied_query(
        history: HistoryTable,
        applied: &Applied,
    ) -> String {
        let history_table = history.full_name();
        format!(
            "
INSERT INTO {history_table}(version, description, content, duration_ms, applied_at)
  VALUES ({}, '{}', '{}', {}, '{}')
  ON DUPLICATE_KEY
  UPDATE
    description = VALUES(description),
    content = VALUES(content),
    duration_ms = VALUES(duration_ms),
    applied_at = VALUES(applied_at);
",
            applied.version(),
            applied.description_ref(),
            applied.content_ref(),
            applied.duration_millis(),
            applied.applied_at(),
        )
    }
}
