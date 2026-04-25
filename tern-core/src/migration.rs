use chrono::{DateTime, Utc};
use futures_core::future::BoxFuture;
use std::fmt::{self, Display, Formatter};

use crate::context::MigrationContext;
use crate::error::TernResult;
use crate::query::Query;

/// One migration within a migration set.
///
/// A `Migration` provides required metadata, the context it needs to be applied,
/// and how to apply it given its context.
pub trait Migration: Send + Sync {
    type Ctx: MigrationContext;

    /// Return a reference to the [`MigrationId`] of this migration.
    fn id(&self) -> &MigrationId;

    /// Resolve the [`Query`] that defines this migration.
    fn resolve_query<'a>(&'a self, ctx: &mut Self::Ctx) -> BoxFuture<'a, TernResult<Query>>;
}

/// Identifier for a migration in a migration set.
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct MigrationId {
    version: i64,
    description: String,
}

impl MigrationId {
    /// New `MigrationId` from values in the filename.
    pub fn new(version: i64, description: String) -> Self {
        Self {
            version,
            description,
        }
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

/// A migration that has been applied to the database.
///
/// This is also the Rust value corresponding to a record in the migration
/// history table.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct AppliedMigration {
    version: i64,
    description: String,
    content: String,
    duration_ms: i64,
    applied_at: DateTime<Utc>,
}

impl AppliedMigration {
    /// New `AppliedMigration`.
    pub fn new(id: &MigrationId, content: String, start: DateTime<Utc>) -> Self {
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

    /// Returns the raw content of the original migration source.
    pub fn content(&self) -> String {
        self.content.clone()
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

/// `MigrationSet` is an ordered collection of migrations.
pub struct MigrationSet<Ctx: ?Sized> {
    migrations: Vec<Box<dyn Migration<Ctx = Ctx>>>,
}

impl<Ctx: MigrationContext> MigrationSet<Ctx> {
    /// Create a new `MigrationSet`.
    ///
    /// This sorts the input by version in ascending order, so inputs need not
    /// be pre-sorted.
    pub fn new<T>(vs: T) -> Self
    where
        T: Into<Vec<Box<dyn Migration<Ctx = Ctx>>>>,
    {
        let mut migrations = vs.into();
        migrations.sort_by_key(|m| m.id().version());
        Self { migrations }
    }

    /// Returns the number of migrations in this migration set.
    pub fn size(&self) -> usize {
        self.migrations.len()
    }

    /// Returns the ordered collection of migration version numbers.
    pub fn versions(&self) -> Vec<i64> {
        self.migrations
            .iter()
            .map(|m| m.id().version())
            .collect()
    }

    /// Returns a tuple of earliest and latest migration version, or `None` if
    /// this migration set is empty.
    pub fn version_range(&self) -> Option<(i64, i64)> {
        let vs = self.versions();
        let earliest = vs.first().cloned()?;
        let latest = vs.last().cloned()?;
        Some((earliest, latest))
    }

    /// Returns whether this migration set is empty.
    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    /// Return the ordered set of migrations in a slice.
    pub fn as_slice(&self) -> &[Box<dyn Migration<Ctx = Ctx>>] {
        self.migrations.as_slice()
    }
}
