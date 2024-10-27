use super::connection::MigrateConn;
use super::history::HistoryTable;
use super::migration::{AppliedMigration, Migration};
use super::source::MigrationSource;
use crate::error::Error;

use futures_core::future::BoxFuture;

/// The runtime for applying a migration set.
///
/// There are two required methods: `apply_tx` and
/// `apply_no_tx`.  These do (respectively, do _not_)
/// run an individual migration within a transaction.
///
/// _Note_: If a migration is not ran in a transaction,
/// an outcome  where the history table reaches an
/// erroneous state is possible: when the migration
/// query itself succeeds but the query to update
/// the history with a new row does not succeed.
pub trait Migrate
where
    Self: Send + Sync,
{
    /// History table to update/interact with.
    type Table: HistoryTable;

    /// Connection to use for migrations.
    type Conn: MigrateConn<ConnTable = Self::Table>;

    /// Get the connection.
    fn conn(&mut self) -> &mut Self::Conn;

    /// Create the history table if it does not exist.
    fn check_history_table(&mut self, table: &Self::Table) -> BoxFuture<'_, Result<(), Error>> {
        let conn = self.conn();
        conn.create_if_not_exists(table)
    }

    /// Get all previously applied migrations.
    fn get_history_table<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::Table,
    ) -> BoxFuture<'a, Result<Vec<AppliedMigration>, Error>> {
        let conn = self.conn();
        conn.select_applied_from(table)
    }

    /// Insert a newly applied migration returning the version.
    fn insert_new_applied<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::Table,
        applied: &'a AppliedMigration,
    ) -> BoxFuture<'a, Result<i64, Error>> {
        let conn = self.conn();
        conn.insert_into(table, applied)
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

/// Empty method for `validate_source`.
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
