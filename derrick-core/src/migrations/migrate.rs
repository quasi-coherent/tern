use super::connection::MigrateConn;
use super::history::HistoryTable;
use super::migration::{AppliedMigration, Migration};
use super::source::MigrationSource;
use crate::error::Error;

use futures_core::future::BoxFuture;

/// Methods for applying a migration set.
///
/// Based on configuration, a migration can opt out
/// of being in a transaction, for instance, concurrent
/// index creation in postgers cannot be ran in a
/// transaction.  The default is to run in a transaction.
///
/// _Note_: If a migration is not ran in a transaction,
/// an outcome  where the history table reaches an
/// erroneous state is possible: when the migration
/// query itself succeeds but the query to update
/// the history with a new row does not succeed.
pub trait Migrate
where
    Self: MigrateConn<ConnTable = Self::Table> + Send,
{
    /// History table to update/interact with.
    type Table: HistoryTable;

    /// Create the history table if it does not exist.
    fn check_history_table(&mut self, table: &Self::Table) -> BoxFuture<'_, Result<(), Error>> {
        <Self as MigrateConn>::create_if_not_exists(self, table)
    }

    /// Get all previously applied migrations.
    fn get_history_table<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::Table,
    ) -> BoxFuture<'a, Result<Vec<AppliedMigration>, Error>> {
        <Self as MigrateConn>::select_applied_from(self, table)
    }

    /// Insert a newly applied migration returning the version.
    fn insert_new_applied<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::Table,
        applied: &'a AppliedMigration,
    ) -> BoxFuture<'a, Result<i64, Error>> {
        <Self as MigrateConn>::insert_into(self, table, applied)
    }

    /// Enforce rules about source migrations.
    fn validate_source(
        source: Vec<MigrationSource>,
        applied: Vec<AppliedMigration>,
    ) -> Result<(), Error> {
        NoValidation::validate(source, applied)
    }

    /// Apply a migration.
    fn apply<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::Table,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>> {
        if migration.no_tx {
            self.apply_no_tx(table, migration)
        } else {
            self.apply_tx(table, migration)
        }
    }

    /// Apply a migration outside a transaction and if
    /// successful, update history.
    fn apply_no_tx<'a, 'c: 'a>(
        &'c mut self,
        table_name: &'a Self::Table,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>>;

    /// Apply a migration and update history in a transaction.
    fn apply_tx<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::Table,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>>;
}

/// Empty `Validation` implementation
#[derive(Clone)]
pub struct NoValidation;

impl NoValidation {
    fn validate(
        _source: Vec<MigrationSource>,
        _applied: Vec<AppliedMigration>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
