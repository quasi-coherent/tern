//! Context needed for applying migrations.
use futures_core::future::{BoxFuture, Future};

use crate::error::TernResult;
use crate::migration::{HistoryTable, MigrationData};
use crate::query::Query;

/// Main context provider for migrations.
///
/// `MigrationContext` is the type of value where user-defined capabilities are
/// inserted.  A migration is always in reference to a `MigrationContext`.
pub trait MigrationContext: Send + Sync {
    /// The type of value used for database interaction.
    ///
    /// Usually a database client of some sort that has had the requisite
    /// queries implemented for it.
    type Exec: MigrationExecutor;

    /// Get a mutable reference to this context's executor.
    fn executor_mut(&mut self) -> &mut Self::Exec;

    /// Get a reference to the database table storing the history of the
    /// associated migration set.
    fn history_table(&self) -> HistoryTable;

    /// Get the latest applied migration.
    ///
    /// Returns `None` if there are no applied migrations.
    fn latest_applied(
        &mut self,
    ) -> BoxFuture<'_, TernResult<Option<MigrationData>>> {
        Box::pin(async move {
            let history = self.history_table();
            let latest = self
                .executor_mut()
                .get_all_applied(history)
                .await?
                .into_iter()
                .fold(None::<MigrationData>, |acc, m| {
                    if acc.as_ref().is_none_or(|a| a.version() < m.version()) {
                        Some(m)
                    } else {
                        acc
                    }
                });
            Ok(latest)
        })
    }
}

/// `MigrationExecutor` is the database client interface for migration
/// operations.
pub trait MigrationExecutor: Send + Sync + 'static {
    /// Send the database query with this executor.
    ///
    /// This is provided naturally and cannot reasonably be overridden.
    fn send(
        &mut self,
        query: &Query,
    ) -> impl Future<Output = TernResult<()>> + Send {
        query.send_with(self)
    }

    /// Send the database query in a transaction.
    fn send_tx(
        &mut self,
        query: &str,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Send the database query _not_ in a transaction.
    fn send_notx(
        &mut self,
        query: &str,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Create the history table.
    fn init_history(
        &mut self,
        history: HistoryTable,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Drop the history table.
    fn drop_history(
        &mut self,
        history: HistoryTable,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Check that the history table exists.
    ///
    /// This is called before every migration run.
    fn check_history(
        &mut self,
        history: HistoryTable,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Return all rows of the history table.
    fn get_all_applied(
        &mut self,
        history: HistoryTable,
    ) -> impl Future<Output = TernResult<Vec<MigrationData>>> + Send;

    /// Insert a newly applied migration into the history table.
    fn insert_applied(
        &mut self,
        history: HistoryTable,
        applied: &MigrationData,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Delete the applied migration from the history table.
    fn delete_applied(
        &mut self,
        history: HistoryTable,
        version: i64,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Insert or update an applied migration in the history table.
    fn upsert_applied(
        &mut self,
        history: HistoryTable,
        applied: &MigrationData,
    ) -> impl Future<Output = TernResult<()>> + Send;
}
