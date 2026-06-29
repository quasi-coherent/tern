//! Common properties of migrations.
use chrono::{DateTime, Utc};
use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use crate::migration::Query;

/// The database table to record migration history in.
///
/// This defaults to "_tern_migrations" so will be created in the default
/// schema according to search path.
#[derive(Debug, Clone, Copy)]
pub struct HistoryTable {
    schema: Option<&'static str>,
    table: &'static str,
}

impl HistoryTable {
    /// New table `table` in the default schema (based on search path).
    pub const fn new(table: &'static str) -> Self {
        Self { schema: None, table }
    }

    /// Specify a schema for the history table.
    pub fn with_schema(self, schema: &'static str) -> Self {
        Self { schema: Some(schema), ..self }
    }

    /// Return the history table.
    pub const fn table(&self) -> &'static str {
        self.table
    }

    /// Return the schema where the history table is.
    pub const fn schema(&self) -> Option<&'static str> {
        self.schema
    }

    /// Returns the history table in the format `<schema>.<table>`.
    pub fn scoped_table(&self) -> String {
        let table = self.table();
        if let Some(schema) = self.schema() {
            return format!("{schema}.{table}");
        }
        table.to_string()
    }
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self { schema: None, table: "_tern_migrations" }
    }
}

impl Display for HistoryTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.scoped_table())
    }
}

/// Identifier for a migration in a migration set.
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct MigrationId {
    version: i64,
    description: Cow<'static, str>,
}

impl MigrationId {
    /// New `MigrationId` from values in the filename.
    pub fn new<T: Into<Cow<'static, str>>>(
        version: i64,
        description: T,
    ) -> Self {
        Self { version, description: description.into() }
    }

    /// Get the migration version.
    pub fn version(&self) -> i64 {
        self.version
    }

    /// Get the migration description.
    pub fn description(&self) -> &str {
        &self.description
    }
}

impl Display for MigrationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "V{}__{}", self.version(), self.description())
    }
}

/// A migration that has been applied to the database, which also can be used to
/// describe applying the inverse of a migration.
///
/// This is also the value that models a record in the history table.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct MigrationData {
    version: i64,
    description: String,
    content: String,
    duration_ms: i64,
    applied_at: DateTime<Utc>,
}

impl MigrationData {
    /// New `MigrationData`.
    pub fn new(id: &MigrationId, query: &Query, start: DateTime<Utc>) -> Self {
        let applied_at = Utc::now();
        let duration_ms = (applied_at - start).num_milliseconds();
        Self {
            version: id.version(),
            description: id.description().to_string(),
            content: query.to_string(),
            duration_ms,
            applied_at,
        }
    }

    /// Returns the ID of the migration that was applied.
    pub fn migration_id(&self) -> MigrationId {
        MigrationId::new(self.version, self.description.clone())
    }

    /// Returns the migration version obtained from the source filename.
    pub fn version(&self) -> i64 {
        self.version
    }

    /// Returns the description of the migration obtained from the source
    /// filename.
    pub fn description(&self) -> String {
        self.description.clone()
    }

    /// Returns a reference to the description of the migration.
    pub fn description_ref(&self) -> &str {
        &self.description
    }

    /// Returns the raw content of the original migration source.
    pub fn content(&self) -> String {
        self.content.clone()
    }

    /// Returns a reference to the raw content of the original migration source.
    pub fn content_ref(&self) -> &str {
        &self.content
    }

    /// Returns the duration in milliseconds of the migration query run.
    pub fn duration_millis(&self) -> i64 {
        self.duration_ms
    }

    /// Returns the UTC timestamp of when the migration was applied.
    pub fn applied_at(&self) -> DateTime<Utc> {
        self.applied_at
    }
}

impl Display for MigrationData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let id = self.migration_id();
        let mut lines = self.content_ref().lines().take(6).collect::<Vec<_>>();
        let truncated = lines.pop().is_some();
        let snip = lines.join("\n");
        let content = if truncated { format!("{snip}...") } else { snip };
        let duration = self
            .duration_ms
            .try_into()
            .map(Duration::from_millis)
            .unwrap_or_default()
            .as_secs_f64();
        let applied_at = self.applied_at;

        write!(
            f,
            r#"
{{
  "id": "{id}",
  "content": "{content}",
  "duration": "{duration}s",
  "applied_at": "{applied_at}",
}}
"#,
        )
    }
}
