use super::history::{HistoryRow, HistoryTable};
use super::migration::AppliedMigration;
use crate::error::Error;

use futures_core::future::BoxFuture;
use std::convert::From;

/// Database connection and query executor.
pub trait MigrateConn
where
    Self: Send + Sync,
{
    /// Connection config type.
    type ConnInfo;

    /// The history table the connection will be used
    /// to interact with.
    type ConnTable: HistoryTable;

    /// Establish a connection from a connection string.
    fn connect(info: Self::ConnInfo) -> BoxFuture<'static, Result<Self, Error>>
    where
        Self: Sized;

    /// Create the history table if it does not exist.
    fn create_if_not_exists(&mut self, table: &Self::ConnTable)
        -> BoxFuture<'_, Result<(), Error>>;

    /// Get the full history table.
    fn select_star_from<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::ConnTable,
    ) -> BoxFuture<'a, Result<Vec<HistoryRow>, Error>>;

    /// Insert a new record into the history table returning the version.
    fn insert_into<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::ConnTable,
        applied: &'a AppliedMigration,
    ) -> BoxFuture<'a, Result<(), Error>>;

    /// Get the full history table but convert the rows
    /// to the type inhabited by pre-inserted migrations.
    fn select_applied_from<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::ConnTable,
    ) -> BoxFuture<'a, Result<Vec<AppliedMigration>, Error>> {
        Box::pin(async move {
            let applied = self
                .select_star_from(table)
                .await?
                .into_iter()
                .map(AppliedMigration::from)
                .collect::<Vec<_>>();

            Ok(applied)
        })
    }
}
