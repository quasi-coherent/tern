//! This module contains the central trait [Migration], which is the interface
//! for a migration source file to be applied in a target database.
//!
//! A [Migration] is built with a specific [MigrationContext] in mind.  It
//! describes how its query is to be built using this context and how that query
//! should be ran against the database.  For Rust migrations, the query is built
//! according to an implementation of [QueryBuilder].  For static SQL migrations,
//! it is simply the file contents.
//!
//! [MigrationContext]: crate::context::MigrationContext
//! [QueryBuilder]: crate::source::query::QueryBuilder
use crate::context::MigrationContext;
use crate::error::TernResult;
use crate::source::query::Query;

use chrono::{DateTime, Utc};
use futures_core::future::BoxFuture;

/// A single migration in a migration set.
pub trait Migration
where
    Self: Send + Sync,
{
    /// A migration context that is sufficient to build this migration.
    type Ctx: MigrationContext;

    /// Get the [MigrationId] for this migration.
    fn migration_id(&self) -> MigrationId;

    /// The raw file content of the migration source file, or when stored as an
    /// applied migration in the history table, it is the query that was ran.
    fn content(&self) -> String;

    /// Whether this migration should not be applied in a database transaction.
    fn no_tx(&self) -> bool;

    /// Produce a future resolving to the migration query when `await`ed.
    fn build<'a>(&'a self, ctx: &'a mut Self::Ctx) -> BoxFuture<'a, TernResult<Query>>;

    /// The migration version.
    fn version(&self) -> i64 {
        self.migration_id().version()
    }

    /// Convert this migration to an [AppliedMigration] assuming that it was
    /// successfully applied.
    fn to_applied(
        &self,
        duration_ms: i64,
        applied_at: DateTime<Utc>,
        content: &str,
    ) -> AppliedMigration {
        AppliedMigration::new(self.migration_id(), content, duration_ms, applied_at)
    }
}

/// A subset of the migrations found in the source.
///
/// This of course need not be a strict subset--a context will return the full
/// set with no target version specified.
pub struct MigrationSet<Ctx: ?Sized> {
    pub migrations: Vec<Box<dyn Migration<Ctx = Ctx>>>,
}

impl<Ctx> MigrationSet<Ctx>
where
    Ctx: MigrationContext,
{
    pub fn new<T>(vs: T) -> MigrationSet<Ctx>
    where
        T: Into<Vec<Box<dyn Migration<Ctx = Ctx>>>>,
    {
        let mut migrations = vs.into();
        migrations.sort_by_key(|m| m.version());
        MigrationSet { migrations }
    }

    /// Number of migrations in the set.
    pub fn len(&self) -> usize {
        self.migrations.len()
    }

    /// The latest version in the set.
    pub fn max(&self) -> Option<i64> {
        self.versions().iter().max().copied()
    }

    /// The set is empty for the requested operation.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Versions present in this migration set.
    pub fn versions(&self) -> Vec<i64> {
        self.migrations
            .iter()
            .map(|m| m.version())
            .collect::<Vec<_>>()
    }

    /// The version/name of migrations in this migration set.
    pub fn migration_ids(&self) -> Vec<MigrationId> {
        self.migrations
            .iter()
            .map(|m| m.migration_id())
            .collect::<Vec<_>>()
    }
}

/// Name and version derived from the migration source filename.
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct MigrationId {
    version: i64,
    description: String,
}

impl MigrationId {
    pub fn new(version: i64, description: String) -> Self {
        Self {
            version,
            description,
        }
    }

    pub fn version(&self) -> i64 {
        self.version
    }

    pub fn description(&self) -> String {
        self.description.clone()
    }
}

impl std::fmt::Display for MigrationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "V{}__{}", self.version, self.description)
    }
}

impl From<AppliedMigration> for MigrationId {
    fn from(value: AppliedMigration) -> Self {
        Self {
            version: value.version,
            description: value.description,
        }
    }
}

/// An `AppliedMigration` is the information about a migration that completed
/// successfully and it is also a row in the schema history table.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct AppliedMigration {
    /// The migration version.
    pub version: i64,
    /// The description of the migration.
    pub description: String,
    /// The contents of the migration file at the time it was applied.
    pub content: String,
    /// How long the migration took to run in milliseconds.
    pub duration_ms: i64,
    /// The timestamp of when the migration was applied.
    pub applied_at: DateTime<Utc>,
}

impl AppliedMigration {
    pub fn new(
        id: MigrationId,
        content: &str,
        duration_ms: i64,
        applied_at: DateTime<Utc>,
    ) -> Self {
        Self {
            version: id.version,
            description: id.description,
            content: content.into(),
            duration_ms,
            applied_at,
        }
    }
}
