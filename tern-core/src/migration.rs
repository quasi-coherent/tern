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
    fn resolve_query<'a>(
        &'a self,
        ctx: &mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>>;
}

/// A dynamically-typed `Migration` for use when a value cannot be statically
/// typed.
pub struct MigrationBox<Ctx: ?Sized>(
    Box<dyn Migration<Ctx = Ctx> + Send + Sync>,
);

impl<Ctx: MigrationContext> MigrationBox<Ctx> {
    /// Wrap the migration `M` in a box.
    pub fn new<M>(mig: M) -> Self
    where
        M: Migration<Ctx = Ctx> + Send + Sync + 'static,
    {
        Self(Box::new(mig))
    }
}

impl<Ctx: MigrationContext> Migration for MigrationBox<Ctx> {
    type Ctx = Ctx;

    fn id(&self) -> &MigrationId {
        self.0.id()
    }

    fn resolve_query<'a>(
        &'a self,
        ctx: &mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        self.0.resolve_query(ctx)
    }
}

/// `MigrationSet` is an ordered collection of migrations.
pub struct MigrationSet<Ctx: ?Sized> {
    migrations: Vec<MigrationBox<Ctx>>,
}

impl<Ctx: MigrationContext> MigrationSet<Ctx> {
    /// Create a new `MigrationSet`.
    ///
    /// This sorts the input by version in ascending order, so inputs need not
    /// be pre-sorted.
    pub fn new<T>(vs: T) -> Self
    where
        T: Into<Vec<MigrationBox<Ctx>>>,
    {
        let mut migrations = vs.into();
        migrations.sort_by_key(|m| m.id().version());
        Self { migrations }
    }

    /// Returns the number of migrations in this migration set.
    pub fn size(&self) -> usize {
        self.migrations.len()
    }

    /// Returns the versions contained in this migration set.
    pub fn versions(&self) -> Vec<i64> {
        self.migrations.iter().map(|m| m.id().version()).collect()
    }

    /// Return the latest version in this set.
    ///
    /// This is `None` when the set is empty.
    pub fn latest(&self) -> Option<i64> {
        self.versions().last().cloned()
    }

    /// Returns whether this migration set is empty.
    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    /// Return the ordered set of migrations in a slice.
    pub fn as_slice(&self) -> &[MigrationBox<Ctx>] {
        self.migrations.as_slice()
    }
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
        Self { version, description }
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
