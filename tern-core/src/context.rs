//! A context for operations on a set of migrations.
//!
//! This defines the core trait [`MigrationContext`].  A `MigrationContext` is
//! responsible for the lifecycle of an operation on some [`MigrationSet`], and
//! where necessary delegating to its associated [`MigrationExecutor`].
use chrono::Utc;
use futures_core::future::BoxFuture;

use crate::error::TernResult;
use crate::executor::{HistoryTable, MigrationExecutor};
use crate::migration::{
    AppliedMigration, Migration, MigrationId, MigrationSet,
};

/// A context for operations on a set of migrations.
///
pub trait MigrationContext: Send + Sync {
    /// The type of value used for database interaction.
    type Exec: MigrationExecutor;

    /// Get a mutable reference to this context's executor.
    fn executor_mut(&mut self) -> &mut Self::Exec;

    /// Return the target table storing the migration history.
    fn history_table(&self) -> HistoryTable;

    /// Return the set of migrations in this context.
    fn migration_set(&self) -> MigrationSet<Self>;

    /// Apply a [`Migration`] with this context.
    ///
    /// This resolves the query for the `Migration`, applies it, then updates the
    /// history table with the applied migration, for any migration whose
    /// associated required context is this one.
    fn apply<'mig, 'ctx: 'mig, M>(
        &'ctx mut self,
        migration: &'mig M,
    ) -> BoxFuture<'mig, TernResult<AppliedMigration>>
    where
        M: Migration<Ctx = Self>,
    {
        Box::pin(async move {
            let start = Utc::now();
            let id = migration.id();
            let history = self.history_table();

            log::trace!("resolving migration {id}");
            let query = migration.resolve_query(self).await?;
            let exec = self.executor_mut();
            log::trace!("applying migration {id}");
            exec.apply(&query).await?;

            let content = query.to_string();
            let applied = AppliedMigration::new(&id, content, start);
            exec.insert_applied(history, &applied).await?;
            log::trace!("applied migration {id}");

            Ok(applied)
        })
    }

    /// Insert a new applied migration into the migration history table.
    fn insert_applied<'mig, 'ctx: 'mig>(
        &'ctx mut self,
        applied: &'mig AppliedMigration,
    ) -> BoxFuture<'mig, TernResult<()>> {
        Box::pin(async move {
            let history = self.history_table();
            self.executor_mut().insert_applied(history, applied).await
        })
    }

    /// Update the migration history with data from an [`AppliedMigration`] or
    /// insert it if the version does not exist in the table.
    fn update_applied<'mig, 'ctx: 'mig>(
        &'ctx mut self,
        applied: &'mig AppliedMigration,
    ) -> BoxFuture<'mig, TernResult<()>> {
        Box::pin(async move {
            let history = self.history_table();
            self.executor_mut().upsert_applied(history, applied).await
        })
    }

    /// Reset the history table to point to the specified version as the latest.
    ///
    /// Note that this works by deleting the row for any previously applied
    /// migration with version greater than the new desired latest version.
    fn reset_last_applied(
        &mut self,
        version: i64,
    ) -> BoxFuture<'_, TernResult<()>> {
        Box::pin(async move {
            let history = self.history_table();
            self.executor_mut().reset_last_applied(history, version).await
        })
    }

    /// Return the collection of all migrations that have been previously
    /// applied, sorted in ascending order by version.
    fn all_applied(
        &mut self,
    ) -> BoxFuture<'_, TernResult<Vec<AppliedMigration>>> {
        Box::pin(async move {
            let history = self.history_table();
            let mut applied =
                self.executor_mut().get_all_applied(history).await?;
            applied.sort_by_key(|m| m.version());
            Ok(applied)
        })
    }

    /// Return the ID of the most recently applied migration.
    ///
    /// This is `None` if no migrations have been applied yet.
    fn last_applied(
        &mut self,
    ) -> BoxFuture<'_, TernResult<Option<MigrationId>>> {
        Box::pin(async move {
            let applied = self.all_applied().await?;
            Ok(applied.last().map(|m| m.migration_id()))
        })
    }

    /// Return applied migrations satisfying the given predicate.
    fn applied_where<'a, 'ctx: 'a, F>(
        &'ctx mut self,
        f: F,
    ) -> BoxFuture<'a, TernResult<Vec<AppliedMigration>>>
    where
        F: FnMut(&AppliedMigration) -> bool + Send + 'a,
    {
        Box::pin(async move {
            let applied = self.all_applied().await?;
            let filtered = applied.into_iter().filter(f).collect();
            Ok(filtered)
        })
    }

    /// Confirm that the migration history table exists in the target database.
    fn check_history_exists(&mut self) -> BoxFuture<'_, TernResult<()>> {
        Box::pin(async move {
            let history = self.history_table();
            self.executor_mut().check_history(history).await
        })
    }

    /// Create the migration history table if it does not exist.
    fn init_history(&mut self) -> BoxFuture<'_, TernResult<()>> {
        Box::pin(async move {
            let history = self.history_table();
            self.executor_mut().create_history_if_not_exists(history).await
        })
    }

    /// Drop the migration history table from the target database.
    fn drop_history(&mut self) -> BoxFuture<'_, TernResult<()>> {
        Box::pin(async move {
            let history = self.history_table();
            self.executor_mut().drop_history(history).await
        })
    }
}
