use super::migration::AppliedMigration;

use chrono::{DateTime, Utc};

/// Interaction with the migration history table.
pub trait HistoryTable
where
    Self: Send + Sync + 'static,
{
    /// Build from a config.
    fn new(info: &HistoryTableInfo) -> Self
    where
        Self: Sized;

    /// The full table path.
    fn table(&self) -> String;

    /// The driver-specifc query for creating
    /// the history table if it does not exist.
    fn create_if_not_exists_query(&self) -> String;

    /// The query to select all rows from the
    /// history table.
    fn select_star_from_query(&self) -> String;

    /// The query to insert a new applied migration.
    fn insert_into_query(&self, applied: &AppliedMigration) -> String;
}

/// Config to create an instance of `HistoryTable`.
#[derive(Debug, Clone, Default)]
pub struct HistoryTableInfo {
    schema: Option<String>,
    table_name: Option<String>,
}

impl HistoryTableInfo {
    pub fn new(schema: Option<String>, table_name: Option<String>) -> Self {
        Self { schema, table_name }
    }

    pub fn set_schema(&mut self, schema: String) -> &Self {
        self.schema = Some(schema);
        self
    }

    pub fn set_table_name(&mut self, table_name: String) -> &Self {
        self.table_name = Some(table_name);
        self
    }

    pub fn schema(&self) -> Option<String> {
        self.schema.clone()
    }

    pub fn table_name(&self) -> Option<String> {
        self.table_name.clone()
    }
}

/// A row in the migration history table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HistoryRow {
    /// The migration version.
    pub version: i64,
    /// The description of the migration.
    pub description: String,
    /// The base64 encoding of the migration file
    /// when it was applied.
    pub content: String,
    /// How long the migration took to run.
    pub duration_sec: i64,
    /// When the applied migration was inserted.
    pub applied_at: DateTime<Utc>,
}

impl From<HistoryRow> for AppliedMigration {
    fn from(v: HistoryRow) -> Self {
        AppliedMigration {
            version: v.version,
            description: v.description,
            content: v.content,
            duration_sec: v.duration_sec,
        }
    }
}
