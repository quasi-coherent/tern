//! Provides [`MigrationContext`], which is the main interface for performing
//! operations with a migration set.
use crate::error::{DatabaseError as _, TernResult};
use crate::source::{AppliedMigration, Migration, MigrationSet};

use chrono::Utc;
use futures_core::future::BoxFuture;
use std::time::Instant;

mod executor;
pub use executor::Executor;

/// `MigrationContext` is the context in which a set of migrations is applied.
pub trait MigrationContext
where
    Self: Send + Sync + 'static,
{
    /// The name of the table in the database that tracks the history of this
    /// migration set.
    ///
    /// By default, the table `_tern_migrations` in the default schema for the
    /// database driver is used.
    const HISTORY_TABLE: &str;

    /// The type for executing queries in a migration run; a connection object.
    type Exec: Executor;

    /// Get a mutable reference to the underlying `Executor`.
    fn executor(&mut self) -> &mut Self::Exec;

    /// Get a migration set given the `target_version`.
    fn migration_set(&self, target_version: Option<i64>) -> MigrationSet<Self>;

    /// For a migration over this context, `apply` builds the migration query,
    /// runs it, then update the schema history table.
    fn apply<'migration, 'conn: 'migration, M>(
        &'conn mut self,
        migration: &'migration M,
    ) -> BoxFuture<'migration, TernResult<AppliedMigration>>
    where
        M: Migration<Ctx = Self> + Send + Sync + ?Sized,
    {
        Box::pin(async move {
            let start = Instant::now();
            let query = M::build(migration, self).await?;
            let executor = self.executor();

            if migration.no_tx() {
                executor
                    .apply_no_tx(&query)
                    .await
                    .void_tern_migration_result(migration)?;
            } else {
                executor
                    .apply_tx(&query)
                    .await
                    .void_tern_migration_result(migration)?;
            }

            let applied_at = Utc::now();
            let duration_ms = start.elapsed().as_millis() as i64;
            let applied = migration.to_applied(duration_ms, applied_at, query.sql());
            executor
                .insert_applied_migration(Self::HISTORY_TABLE, &applied)
                .await?;

            Ok(applied)
        })
    }

    /// Gets the version of the most recently applied migration.
    fn latest_version(&mut self) -> BoxFuture<'_, TernResult<Option<i64>>> {
        Box::pin(async move {
            let latest = self
                .executor()
                .get_all_applied(Self::HISTORY_TABLE)
                .await?
                .into_iter()
                .fold(None, |acc, m| match acc {
                    None => Some(m.version),
                    Some(v) if m.version > v => Some(m.version),
                    _ => acc,
                });

            Ok(latest)
        })
    }

    /// Get all previously applied migrations.
    fn previously_applied(&mut self) -> BoxFuture<'_, TernResult<Vec<AppliedMigration>>> {
        Box::pin(self.executor().get_all_applied(Self::HISTORY_TABLE))
    }

    /// Check that the history table exists and create it if not.
    fn check_history_table(&mut self) -> BoxFuture<'_, TernResult<()>> {
        Box::pin(
            self.executor()
                .create_history_if_not_exists(Self::HISTORY_TABLE),
        )
    }

    /// Drop the history table if requested.
    fn drop_history_table(&mut self) -> BoxFuture<'_, TernResult<()>> {
        Box::pin(self.executor().drop_history(Self::HISTORY_TABLE))
    }

    /// Insert an applied migration.
    fn insert_applied<'migration, 'conn: 'migration>(
        &'conn mut self,
        applied: &'migration AppliedMigration,
    ) -> BoxFuture<'migration, TernResult<()>> {
        Box::pin(
            self.executor()
                .insert_applied_migration(Self::HISTORY_TABLE, applied),
        )
    }

    /// Upsert applied migrations.
    fn upsert_applied<'migration, 'conn: 'migration>(
        &'conn mut self,
        applied: &'migration AppliedMigration,
    ) -> BoxFuture<'migration, TernResult<()>> {
        Box::pin(
            self.executor()
                .upsert_applied_migration(Self::HISTORY_TABLE, applied),
        )
    }
}
