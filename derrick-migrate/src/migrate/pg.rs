/// History table syntax for postgres.
#[derive(Clone)]
pub struct PgHistoryTableInfo {
    table: String,
}

impl PgHistoryTableInfo {
    pub fn new(table: String) -> Self {
        Self { table }
    }

    pub fn table(&self) -> String {
        self.table.clone()
    }

    pub fn create_if_not_exists_query(&self) -> String {
        let name = self.table();
        let query = format!(
            r#"
CREATE TABLE IF NOT EXISTS {}(
  version bigint PRIMARY KEY,
  description text NOT NULL,
  content text NOT NULL,
  duration_ms double precision NOT NULL,
  applied_at timestamptz NOT NULL DEFAULT now()
);
"#,
            name,
        );

        query
    }

    pub fn select_star_from_query(&self) -> String {
        let name = self.table();
        let query = format!(
            r#"
SELECT
  version,
  description,
  content,
  duration_ms,
  applied_at
FROM
  {}
ORDER BY
  version;
"#,
            name,
        );

        query
    }
}
