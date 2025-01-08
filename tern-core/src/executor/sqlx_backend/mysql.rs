use sqlx::MySql;

use super::pool::SqlxExecutor;
use crate::migration::{AppliedMigration, Query, QueryRepository};

/// Specialization of the generic context for `sqlx::Pool` to `sqlx::MySqlPool`.
pub type SqlxMySqlExecutor = SqlxExecutor<MySql, SqlxMySqlQueryRepo>;

/// The schema history table queries for mysql.
#[derive(Debug, Clone)]
pub struct SqlxMySqlQueryRepo;

impl QueryRepository for SqlxMySqlQueryRepo {
    fn create_history_if_not_exists_query(history_table: &str) -> Query {
        let sql = format!(
            "
CREATE TABLE IF NOT EXISTS {history_table}(
  version bigint PRIMARY KEY,
  description text NOT NULL,
  content text NOT NULL,
  duration_ms bigint NOT NULL,
  applied_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);
"
        );

        Query::new(sql)
    }

    fn drop_history_query(history_table: &str) -> Query {
        let sql = format!("DROP TABLE IF EXISTS {history_table};");

        Query::new(sql)
    }

    fn insert_into_history_query(history_table: &str, _: &AppliedMigration) -> Query {
        // With `sqlx` we're not going to use the `AppliedMigration`, the values
        // will get in the query by `bind`ing them.
        let sql = format!(
            "
INSERT INTO {history_table}(version, description, content, duration_ms, applied_at)
  VALUES (?, ?, ?, ?, ?);
"
        );

        Query::new(sql)
    }

    fn select_star_from_history_query(history_table: &str) -> Query {
        let sql = format!(
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
        );

        Query::new(sql)
    }

    fn upsert_history_query(history_table: &str, _: &AppliedMigration) -> Query {
        let sql = format!(
            "
INSERT INTO {history_table}(version, description, content, duration_ms, applied_at)
  VALUES (?, ?, ?, ?, ?)
  ON DUPLICATE_KEY
  UPDATE
    description = VALUES(description),
    content = VALUES(content),
    duration_ms = VALUES(duration_ms),
    applied_at = VALUES(applied_at)
"
        );

        Query::new(sql)
    }
}
