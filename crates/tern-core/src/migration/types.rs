//! Types related to `Migration`s.
use chrono::{DateTime, Utc};
use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};

/// A type that owns the ID of a migration within a set of migrations.
///
/// This should not need to be used directly except maybe when not using macros.
pub trait HasMigrationId {
    /// Return a reference to the migration ID.
    fn id_ref(&self) -> &MigrationId;
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
#[cfg_attr(feature = "sqlx_derive", derive(sqlx::FromRow))]
pub struct Applied {
    version: i64,
    description: String,
    content: String,
    duration_ms: i64,
    applied_at: DateTime<Utc>,
}

impl Applied {
    /// New `Applied`.
    pub fn new(
        id: &MigrationId,
        content: String,
        start: DateTime<Utc>,
    ) -> Self {
        let applied_at = Utc::now();
        let duration_ms = (applied_at - start).num_milliseconds();
        Self {
            version: id.version(),
            description: id.description().into(),
            content,
            duration_ms,
            applied_at,
        }
    }

    /// Returns the [`MigrationId`] of the migration that was applied.
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
