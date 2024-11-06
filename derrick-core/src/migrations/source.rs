use super::migrate::Migrate;
use super::migration::Migration;
use crate::error::Error;

use futures_core::future::BoxFuture;
use std::fmt::Write;

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
    fn build_query<'a>(
        &'a self,
        runtime: &'a mut Self::Runtime,
    ) -> BoxFuture<'a, Result<MigrationQuery, Error>>;

    fn resolve<'a, 'c: 'a>(
        &'c self,
        runtime: &'c mut Self::Runtime,
        source: &'a MigrationSource,
    ) -> BoxFuture<'a, Result<Migration, Error>> {
        Box::pin(async move {
            let query = self.build_query(runtime).await?;
            Migration::new(source, query)
        })
    }
}

/// A query and if it should not run in a transaction.
#[derive(Clone)]
pub struct MigrationQuery {
    sql: String,
    no_tx: bool,
}

impl MigrationQuery {
    pub fn new(sql: String, no_tx: bool) -> Self {
        Self { sql, no_tx }
    }

    pub fn sql(&self) -> &str {
        &self.sql
    }

    pub fn no_tx(&self) -> bool {
        self.no_tx
    }

    /// No-tx queries with multiple statements need to be broken up
    /// into individual statements.  TODO(qcoh): This could be less
    /// brittle probably.
    pub fn statements(&self) -> Result<Vec<String>, Error> {
        let sql = &self.sql;
        let mut statements = Vec::new();
        sql.lines()
            .into_iter()
            .try_fold(String::new(), |mut buf, line| {
                let line = line.trim();
                // A comment or a not-a-statement-terminator line
                // is a line belonging in this statement.  Otherwise
                // it's the last line of the statement.
                if line.starts_with("--") || !line.ends_with(";") {
                    writeln!(buf, "{}", line)?;
                    return Ok::<String, std::fmt::Error>(buf);
                };
                writeln!(buf, "{}", line)?;
                // Last line of the statement. Push statement to
                // collection and reset buffer.
                statements.push(buf);
                Ok(String::new())
            })?;

        Ok(statements)
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
