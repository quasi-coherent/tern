use derrick_core::error::Error;
use derrick_core::prelude::*;
use derrick_core::reexport::BoxFuture;
use derrick_core::types::{AppliedMigration, Migration, MigrationSource};

/// Describes the main operations when used in
/// practice.
///
/// The derive macro `Runtime` generates
/// an implementation for anything that implements
/// `Migrate`.
pub trait Runner
where
    Self: Migrate,
{
    /// It should own the migration source directory
    /// parsed into a standard format without needing
    /// a database connection.
    fn sources() -> Vec<MigrationSource>;

    /// It should be able to produce a vector of futures
    /// when given a connection, where the collection is
    /// the set of migrations that have not been applied.
    /// `await`ing this is the input to a migration run.
    fn unapplied<'a, 'c: 'a>(&'c mut self) -> BoxFuture<'a, Result<Vec<Migration>, Error>>;

    /// Lift the underlying `initialize`.
    fn new_runner(
        db_url: String,
        history: <Self as Migrate>::History,
        data: <Self as Migrate>::Init,
    ) -> BoxFuture<'static, Result<Self, Error>>
    where
        Self: Sized,
    {
        <Self as Migrate>::initialize(db_url, history, data)
    }

    /// Validate this set of migrations against the history table.
    fn validate<'a, 'c: 'a>(&'c mut self) -> BoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            let sources = Self::sources().clone();
            let applied = self.get_all_applied().await?.clone();

            <Self as Migrate>::validate_source(sources, applied)
        })
    }

    /// The main method.  It calls the collection of
    /// future migrations from the source directory, resolves
    /// them, and then applies them.
    fn run<'a, 'c: 'a>(&'c mut self) -> BoxFuture<'a, Result<Vec<AppliedMigration>, Error>> {
        Box::pin(async move {
            self.check_history_table().await?;
            let unapplied = self.unapplied().await?;
            let mut applied = Vec::new();

            for migration in unapplied.iter() {
                let new_applied = self.apply(migration).await?;
                applied.push(new_applied);
            }

            Ok(applied)
        })
    }

    /// This applies a set of migrations provided by
    /// some method returning a set of unresolved migrations.
    fn run_with<'a, 'c: 'a, F>(
        &'c mut self,
        callback: F,
    ) -> BoxFuture<'a, Result<Vec<AppliedMigration>, Error>>
    where
        for<'b> F:
            FnOnce(&'b mut Self) -> BoxFuture<'b, Result<Vec<Migration>, Error>> + Send + Sync + 'b,
    {
        Box::pin(async move {
            let migrations = callback(self).await?;
            let mut applied = Vec::new();
            for migration in migrations.iter() {
                let new_applied = self.apply(migration).await?;
                applied.push(new_applied);
            }

            Ok(applied)
        })
    }
}
