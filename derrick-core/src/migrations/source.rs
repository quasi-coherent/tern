use super::migrate::Migrate;
use super::migration::Migration;
use crate::error::Error;

use futures_core::future::BoxFuture;

/// This type can produce the query for the migration.
///
/// The `M` is *the* migration runner, so it
/// needs to have the union of all the capabilities
/// needed for any one migration.
pub trait QueryBuilder
where
    Self: Send + Sync,
{
    type Runtime: Migrate;

    /// Minimum required for Rust migrations.
    fn build_query(
        &self,
        migrate: &mut Self::Runtime,
    ) -> BoxFuture<'_, Result<MigrationQuery<'_>, Error>>;

    fn resolve<'a, 'c: 'a>(
        &'c self,
        migrate: &'c mut Self::Runtime,
        source: &'a MigrationSource,
    ) -> BoxFuture<'a, Result<Migration, Error>> {
        Box::pin(async move {
            let query = self.build_query(migrate).await?;
            Ok(Migration::new(source, &query))
        })
    }
}

/// A future resolved to the final migration.
pub struct FutureMigration<'a> {
    pub version: i64,
    pub migration: BoxFuture<'a, Result<Migration, Error>>,
}

impl<'a, 'c: 'a> FutureMigration<'a> {
    pub fn build<M, Q>(
        runtime: &'c mut M,
        builder: &'c Q,
        source: &'a MigrationSource,
    ) -> BoxFuture<'a, FutureMigration<'a>>
    where
        M: Migrate,
        Q: QueryBuilder<Runtime = M>,
    {
        Box::pin(async move {
            let version = source.version;
            let migration = builder.resolve(runtime, source);
            FutureMigration { version, migration }
        })
    }
}

/// A query and if it should not run in a transaction.
#[derive(Clone)]
pub struct MigrationQuery<'a> {
    sql: &'a str,
    no_tx: bool,
}

impl<'a> MigrationQuery<'a> {
    pub fn new(sql: &'a str, no_tx: bool) -> Self {
        Self { sql, no_tx }
    }

    pub fn sql(&self) -> &'a str {
        self.sql
    }

    pub fn no_tx(&self) -> bool {
        self.no_tx
    }
}

/// The static values parsed from the name
/// and content of the migration source file.
#[derive(Debug, Clone)]
pub struct MigrationSource {
    /// Version parsed from the migration filename.
    pub version: i64,
    /// The description parsed from the filename.
    pub description: String,
    /// The actual content of the file.
    pub content: String,
}

impl MigrationSource {
    pub fn new(version: i64, description: String, content: String) -> MigrationSource {
        MigrationSource {
            version,
            description,
            content,
        }
    }

    /// Arrange a migration set in ascending order by version.
    pub fn order_by_asc(mut source: Vec<MigrationSource>) -> Vec<MigrationSource> {
        source.sort_by_key(|m| m.version);
        source
    }
}
