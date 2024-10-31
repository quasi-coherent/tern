use super::history::{HistoryRow, HistoryTable};
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
    /// A `Migrate` has to manage the history table.
    type History: HistoryTable + Clone + Send + Sync;

    /// Additional data needed to initialize.
    type Init: Clone + Send + Sync;

    /// Create a new value.
    fn initialize(
        db_url: String,
        history: Self::History,
        data: Self::Init,
    ) -> BoxFuture<'static, Result<Self, Error>>
    where
        Self: Sized;

    /// Create the history table if it does not exist.
    fn check_history_table(&mut self) -> BoxFuture<'_, Result<(), Error>>;

    /// Get the full history table.
    fn get_history_rows(&mut self) -> BoxFuture<'_, Result<Vec<HistoryRow>, Error>>;

    /// Insert a newly applied migration.
    fn insert_new_applied<'a, 'c: 'a>(
        &'c mut self,
        applied: &'a AppliedMigration,
    ) -> BoxFuture<'a, Result<(), Error>>;

    /// Apply a migration outside a transaction and if
    /// successful, update history.
    fn apply_no_tx<'a, 'c: 'a>(
        &'c mut self,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>>;

    /// Apply a migration and update history in a transaction.
    fn apply_tx<'a, 'c: 'a>(
        &'c mut self,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>>;

    /// Get the full history table but convert the rows
    /// to the type inhabited by pre-inserted migrations.
    fn get_all_applied<'a, 'c: 'a>(
        &'c mut self,
    ) -> BoxFuture<'a, Result<Vec<AppliedMigration>, Error>> {
        Box::pin(async move {
            let applied = self
                .get_history_rows()
                .await?
                .into_iter()
                .map(AppliedMigration::from)
                .collect::<Vec<_>>();

            Ok(applied)
        })
    }

    /// Get the most recent applied migration version.
    fn current_version(&mut self) -> BoxFuture<'_, Result<Option<i64>, Error>> {
        Box::pin(async move {
            let current = self
                .get_all_applied()
                .await?
                .into_iter()
                .fold(None::<i64>, |acc, m| match acc {
                    None => Some(m.version),
                    Some(v) if m.version > v => Some(m.version),
                    _ => acc,
                });

            Ok(current)
        })
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
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>> {
        if migration.no_tx {
            self.apply_no_tx(migration)
        } else {
            self.apply_tx(migration)
        }
    }
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
