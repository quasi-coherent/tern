use super::migration::AppliedMigration;

use chrono::{DateTime, Utc};
use std::convert::From;

/// Interaction with the migration history table.
pub trait HistoryTable
where
    Self: Send + Sync + Clone,
{
    /// Build from schema/table names.
    fn new(info: &HistoryTableInfo) -> Self
    where
        Self: Sized;

    /// Get the full object `{schema}.{table_name}`.
    fn table(&self) -> String;

    fn create_if_not_exists_query(&self) -> String;
    fn select_star_from_query(&self) -> String;
    fn insert_into_query(&self, applied: &AppliedMigration) -> String;
}

/// Config to create an instance of `HistoryTable`.
#[derive(Debug, Clone)]
pub struct HistoryTableInfo {
    /// If none, defaults to what the connection thinks.
    schema: Option<String>,
    /// Optional, but by now has been set to the default
    /// "_derrick_migrations" if it wasn't specified.
    table_name: String,
}

impl Default for HistoryTableInfo {
    fn default() -> Self {
        Self {
            schema: None,
            table_name: "_derrick_migrations".to_string(),
        }
    }
}

impl HistoryTableInfo {
    pub fn new(table_name: String) -> Self {
        Self {
            table_name,
            ..Default::default()
        }
    }

    pub fn set_table_name_if_some(mut self, table_name: Option<String>) -> Self {
        if let Some(t) = table_name {
            self.table_name = t;
        }
        self
    }

    pub fn set_schema_if_some(mut self, schema: Option<String>) -> Self {
        if let Some(s) = schema {
            self.schema = Some(s);
        }
        self
    }

    pub fn schema(&self) -> Option<String> {
        self.schema.clone()
    }

    pub fn table_name(&self) -> String {
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
