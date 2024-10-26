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
  applied_at timestamptz NOT NULL DEFAULT now(),
  content text NOT NULL,
  checksum text NOT NULL,
  duration_sec bigint NOT NULL
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
  applied_at,
  content,
  checksum,
  duration_sec
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
