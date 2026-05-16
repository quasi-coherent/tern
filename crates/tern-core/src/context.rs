//! Context for applying migrations.
use futures_core::future::BoxFuture;
use std::ops::{Deref, DerefMut};

use crate::error::TernResult;
use crate::executor::{Executor, HistoryTable};
use crate::migration::{Applied, MigrationId};

/// Context needed for a migration or set of migrations to be operated on.
pub trait MigrationContext: Send + Sync {
    /// The type of value used for database interaction.
    ///
    /// Usually a database client of some sort that has had the requisite
    /// queries implemented for it.
    type Exec: Executor;

    /// Get a mutable reference to this context's executor.
    fn executor_mut(&mut self) -> &mut Self::Exec;

    /// Get a reference to the database table storing the history of the
    /// associated migration set.
    fn history_table(&self) -> HistoryTable;

    /// Insert a new applied migration into the migration history table.
    fn insert_applied<'mig, 'ctx: 'mig>(
        &'ctx mut self,
        applied: &'mig Applied,
    ) -> BoxFuture<'mig, TernResult<()>> {
        Box::pin(async move {
            let history = self.history_table();
            self.executor_mut().insert_applied(history, applied).await
        })
    }

    /// Update the history table with data from `applied`.
    fn update_applied<'mig, 'ctx: 'mig>(
        &'ctx mut self,
        applied: &'mig Applied,
    ) -> BoxFuture<'mig, TernResult<()>> {
        Box::pin(async move {
            let history = self.history_table();
            self.executor_mut().upsert_applied(history, applied).await
        })
    }

    /// Delete the applied migration from the history table.
    fn delete_applied(
        &mut self,
        version: i64,
    ) -> BoxFuture<'_, TernResult<()>> {
        Box::pin(async move {
            let history = self.history_table();
            self.executor_mut().delete_applied(history, version).await
        })
    }

    /// Returns the collection of all migrations that have been previously
    /// applied, sorted in ascending order by version.
    fn all_applied(&mut self) -> BoxFuture<'_, TernResult<Vec<Applied>>> {
        Box::pin(async move {
            let history = self.history_table();
            let mut applied =
                self.executor_mut().get_all_applied(history).await?;
            applied.sort_by_key(|m| m.version());
            Ok(applied)
        })
    }

    /// Returns the ID of the most recently applied migration.
    ///
    /// This is `None` if no migrations have been applied yet.
    fn last_applied_id(
        &mut self,
    ) -> BoxFuture<'_, TernResult<Option<MigrationId>>> {
        Box::pin(async move {
            let applied = self.all_applied().await?;
            Ok(applied.last().map(|m| m.migration_id()))
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

impl<Ctx, D> MigrationContext for D
where
    D: DerefMut<Target = Ctx> + Send + Sync,
    for<'d> Ctx: MigrationContext + 'd,
{
    type Exec = Ctx::Exec;

    fn executor_mut(&mut self) -> &mut Self::Exec {
        self.deref_mut().executor_mut()
    }

    fn history_table(&self) -> HistoryTable {
        self.deref().history_table()
    }
}

/// Extension to `MigrationContext` to put non-object safe methods.
pub trait MigrationContextExt: MigrationContext {
    /// Returns applied migrations satisfying the given predicate.
    fn applied_where<'a, 'ctx: 'a, F>(
        &'ctx mut self,
        f: F,
    ) -> BoxFuture<'a, TernResult<Vec<Applied>>>
    where
        F: FnMut(&Applied) -> bool + Send + 'a,
    {
        Box::pin(async move {
            let applied = self.all_applied().await?;
            let filtered = applied.into_iter().filter(f).collect();
            Ok(filtered)
        })
    }
}

impl<Ctx: MigrationContext> MigrationContextExt for Ctx {}
