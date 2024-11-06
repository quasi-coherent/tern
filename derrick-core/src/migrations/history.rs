use super::migration::AppliedMigration;

use chrono::{DateTime, Utc};
use std::convert::From;

/// Interaction with the migration history table.
pub trait HistoryTable
where
    Self: Send + Sync + Clone,
{
    /// Build from the table name.
    fn new(options: &HistoryTableOptions) -> Self
    where
        Self: Sized;

    /// Get the table name.
    fn table(&self) -> String;

    fn create_if_not_exists_query(&self) -> String;
    fn select_star_from_query(&self) -> String;
    fn insert_into_query(&self, applied: &AppliedMigration) -> String;
}

/// Config for something that is a `HistoryTable`.
#[derive(Debug, Clone)]
pub struct HistoryTableOptions {
    /// Optional, but by now has been set to the default
    /// "_derrick_migrations" if it wasn't specified.
    name: String,
}

impl Default for HistoryTableOptions {
    fn default() -> Self {
        Self {
            name: "_derrick_migrations".to_string(),
        }
    }
}

impl HistoryTableOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_name(mut self, name: Option<String>) -> Self {
        let Some(n) = name else {
            return self;
        };
        self.name = n;
        self
    }

    pub fn name(&self) -> String {
        self.name.clone()
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
    pub duration_ms: i64,
    /// When the applied migration was inserted.
    pub applied_at: DateTime<Utc>,
}

impl From<HistoryRow> for AppliedMigration {
    fn from(v: HistoryRow) -> Self {
        AppliedMigration {
            version: v.version,
            description: v.description,
            content: v.content,
            duration_ms: v.duration_ms,
        }
    }
}
