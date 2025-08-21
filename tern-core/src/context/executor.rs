//! Provides [Executor], which represents a low-level database connection.
use crate::error::TernResult;
use crate::source::{AppliedMigration, Query, QueryRepository};

use futures_core::future::Future;

/// `Executor` is the query interface for database migration operations.
pub trait Executor
where
    Self: Send + Sync + 'static,
{
    /// The type of value that can produce queries for the history table of this
    /// migration set.
    type Queries: QueryRepository;

    /// Apply the [Query] for the migration in a transaction.
    ///
    /// [Query]: crate::source::Query
    fn apply_tx(&mut self, query: &Query) -> impl Future<Output = TernResult<()>> + Send;

    /// Apply the [Query] for the migration _not_ in a transaction.
    ///
    /// [Query]: crate::source::Query
    fn apply_no_tx(&mut self, query: &Query) -> impl Future<Output = TernResult<()>> + Send;

    /// `CREATE IF NOT EXISTS` the history table.
    fn create_history_if_not_exists(
        &mut self,
        history_table: &str,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// `DROP` the history table.
    fn drop_history(&mut self, history_table: &str) -> impl Future<Output = TernResult<()>> + Send;

    /// Get the complete history of applied migrations.
    fn get_all_applied(
        &mut self,
        history_table: &str,
    ) -> impl Future<Output = TernResult<Vec<AppliedMigration>>> + Send;

    /// Insert an applied migration into the history table.
    fn insert_applied_migration(
        &mut self,
        history_table: &str,
        applied: &AppliedMigration,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Update or insert an applied migration.
    fn upsert_applied_migration(
        &mut self,
        history_table: &str,
        applied: &AppliedMigration,
    ) -> impl Future<Output = TernResult<()>> + Send;
}
