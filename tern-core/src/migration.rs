//! This module contains types and traits related to the migration files.
//!
//! * [`Migration`] is the abstract representation of what is built from a
//!   migration file.
//! * [`QueryBuilder`] is the recipe for building the query for a migration.
//! * [`MigrationSource`] is the ability to produce the set of migrations, a
//!   [`MigrationSet`], for a particular context in order to be ran in that
//!   context.
//! * [`MigrationContext`] is the core type.  It has an associated [`Executor`]
//!   and it can produce the migrations from source.  Combined, it has the full
//!   functionality of the migration tool.
//!
//! Generally these shouldn't be implemented; use the corresponding derive macro
//! instead.
use crate::error::{DatabaseError as _, TernResult};

use chrono::{DateTime, Utc};
use futures_core::{Future, future::BoxFuture};
use std::{fmt::Write, time::Instant};

/// The context in which a migration run occurs.
pub trait MigrationContext
where
    Self: Send + Sync + 'static,
{
    /// The name of the table in the database that tracks the history of this
    /// migration set.
    ///
    /// It defaults to `_tern_migrations` in the default schema for the
    /// database driver if using the derive macro for this trait.
    const HISTORY_TABLE: &str;

    /// The type for executing queries in a migration run.
    type Exec: Executor;

    /// A reference to the underlying `Executor`.
    fn executor(&mut self) -> &mut Self::Exec;

    ///
    fn migration_set(&self, last_applied: Option<i64>) -> MigrationSet<Self>;

    /// For a migration that is capable of building its query in this migration
    /// context, this builds the query, applies the migration, then updates the
    /// schema history table after.
    fn apply<'migration, 'conn: 'migration, M>(
        &'conn mut self,
        migration: &'migration M,
    ) -> BoxFuture<'migration, TernResult<AppliedMigration>>
    where
        M: Migration<Ctx = Self> + Send + Sync + ?Sized,
    {
        Box::pin(async move {
            let start = Instant::now();
            let query = M::build(migration, self).await?;
            let executor = self.executor();

            if migration.no_tx() {
                executor
                    .apply_no_tx(&query)
                    .await
                    .void_tern_migration_result(migration)?;
            } else {
                executor
                    .apply_tx(&query)
                    .await
                    .void_tern_migration_result(migration)?;
            }

            let applied_at = Utc::now();
            let duration_ms = start.elapsed().as_millis() as i64;
            let applied = migration.to_applied(duration_ms, applied_at, query.sql());
            executor
                .insert_applied_migration(Self::HISTORY_TABLE, &applied)
                .await?;

            Ok(applied)
        })
    }

    /// Gets the version of the most recently applied migration.
    fn latest_version(&mut self) -> BoxFuture<'_, TernResult<Option<i64>>> {
        Box::pin(async move {
            let latest = self
                .executor()
                .get_all_applied(Self::HISTORY_TABLE)
                .await?
                .into_iter()
                .fold(None, |acc, m| match acc {
                    None => Some(m.version),
                    Some(v) if m.version > v => Some(m.version),
                    _ => acc,
                });

            Ok(latest)
        })
    }

    /// Get all previously applied migrations.
    fn previously_applied(&mut self) -> BoxFuture<'_, TernResult<Vec<AppliedMigration>>> {
        Box::pin(self.executor().get_all_applied(Self::HISTORY_TABLE))
    }

    /// Check that the history table exists and create it if not.
    fn check_history_table(&mut self) -> BoxFuture<'_, TernResult<()>> {
        Box::pin(
            self.executor()
                .create_history_if_not_exists(Self::HISTORY_TABLE),
        )
    }

    /// Drop the history table if requested.
    fn drop_history_table(&mut self) -> BoxFuture<'_, TernResult<()>> {
        Box::pin(self.executor().drop_history(Self::HISTORY_TABLE))
    }

    /// Insert an applied migration.
    fn insert_applied<'migration, 'conn: 'migration>(
        &'conn mut self,
        applied: &'migration AppliedMigration,
    ) -> BoxFuture<'migration, TernResult<()>> {
        Box::pin(
            self.executor()
                .insert_applied_migration(Self::HISTORY_TABLE, applied),
        )
    }

    /// Upsert applied migrations.
    fn upsert_applied<'migration, 'conn: 'migration>(
        &'conn mut self,
        applied: &'migration AppliedMigration,
    ) -> BoxFuture<'migration, TernResult<()>> {
        Box::pin(
            self.executor()
                .upsert_applied_migration(Self::HISTORY_TABLE, applied),
        )
    }
}

/// The "executor" type for the database backend ultimately responsible for
/// issuing migration and schema history queries.
pub trait Executor
where
    Self: Send + Sync + 'static,
{
    /// The type of value that can produce queries for the history table of this
    /// migration set.
    type Queries: QueryRepository;

    /// Apply the `Query` for the migration in a transaction.
    fn apply_tx(&mut self, query: &Query) -> impl Future<Output = TernResult<()>> + Send;

    /// Apply the `Query` for the migration _not_ in a transaction.
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

/// A type that has a library of "administrative" queries that are needed during
/// a migration run.
pub trait QueryRepository {
    /// The query that creates the schema history table or does nothing if it
    /// already exists.
    fn create_history_if_not_exists_query(history_table: &str) -> Query;

    /// The query that drops the history table if requested.
    fn drop_history_query(history_table: &str) -> Query;

    /// The query to update the schema history table with an applied migration.
    fn insert_into_history_query(history_table: &str, applied: &AppliedMigration) -> Query;

    /// The query to return all rows from the schema history table.
    fn select_star_from_history_query(history_table: &str) -> Query;

    /// Query to insert or update a record in the history table.
    fn upsert_history_query(history_table: &str, applied: &AppliedMigration) -> Query;
}

/// A single migration in a migration set.
pub trait Migration
where
    Self: Send + Sync,
{
    /// A migration context that is sufficient to build this migration.
    type Ctx: MigrationContext;

    /// Get the `MigrationId` for this migration.
    fn migration_id(&self) -> MigrationId;

    /// The raw file content of the migration source file, or when stored as an
    /// applied migration in the history table, it is the query that was ran.
    fn content(&self) -> String;

    /// Whether this migration should not be applied in a database transaction.
    fn no_tx(&self) -> bool;

    /// Produce a future resolving to the migration query when `await`ed.
    fn build<'a>(&'a self, ctx: &'a mut Self::Ctx) -> BoxFuture<'a, TernResult<Query>>;

    /// The migration version.
    fn version(&self) -> i64 {
        self.migration_id().version()
    }

    /// Convert this migration to an [`AppliedMigration`] assuming that it was
    /// successfully applied.
    fn to_applied(
        &self,
        duration_ms: i64,
        applied_at: DateTime<Utc>,
        content: &str,
    ) -> AppliedMigration {
        AppliedMigration::new(self.migration_id(), content, duration_ms, applied_at)
    }
}

// /// A type that is used to collect a [`MigrationSet`] -- migrations that are not
// /// applied yet -- which is used as the input to runner commands.
// pub trait MigrationSource {
//     /// A context that the set of migrations returned by `migration_set` would
//     /// need in order to be applied.
//     type Ctx: MigrationContext;

//     /// The set of migrations since `last_applied`.
//     fn migration_set(&self, last_applied: Option<i64>) -> MigrationSet<Self::Ctx>;
// }

/// The `Migration`s derived from the files in the source directory that need to
/// be applied.
pub struct MigrationSet<Ctx: ?Sized> {
    pub migrations: Vec<Box<dyn Migration<Ctx = Ctx>>>,
}

impl<Ctx> MigrationSet<Ctx>
where
    Ctx: MigrationContext,
{
    pub fn new<T>(vs: T) -> MigrationSet<Ctx>
    where
        T: Into<Vec<Box<dyn Migration<Ctx = Ctx>>>>,
    {
        let mut migrations = vs.into();
        migrations.sort_by_key(|m| m.version());
        MigrationSet { migrations }
    }

    /// Number of migrations in the set.
    pub fn len(&self) -> usize {
        self.migrations.len()
    }

    /// Versions present in this migration set.
    pub fn versions(&self) -> Vec<i64> {
        self.migrations
            .iter()
            .map(|m| m.version())
            .collect::<Vec<_>>()
    }

    /// The version/name of migrations in this migration set.
    pub fn migration_ids(&self) -> Vec<MigrationId> {
        self.migrations
            .iter()
            .map(|m| m.migration_id())
            .collect::<Vec<_>>()
    }

    /// The latest version in the set.
    pub fn max(&self) -> Option<i64> {
        self.versions().iter().max().copied()
    }

    /// The set is empty for the requested operation.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A helper trait for [`Migration`].
///
/// With the derive macros, the user's responsibility is to implement this for
/// a Rust migration, and the proc macro uses it to build an implementation of
/// [`Migration`].
pub trait QueryBuilder {
    /// The context for running the migration this query is for.
    type Ctx: MigrationContext;

    /// Asynchronously produce the migration query.
    fn build(&self, ctx: &mut Self::Ctx) -> impl Future<Output = TernResult<Query>> + Send;
}

/// A SQL query.
#[derive(Debug, Clone)]
pub struct Query(pub(crate) String);

impl Query {
    pub fn new(sql: String) -> Self {
        Self(sql)
    }

    fn sanitize(&self) -> String {
        use regex::Regex;
        let block_comment = Regex::new(r"\/\*(?s).*?(?-s)\*\/").unwrap();
        let sql = self
            .sql()
            .trim()
            .lines()
            .filter(|line| {
                let line = line.trim();
                !line.starts_with("--") || line.is_empty()
            })
            .map(|line| {
                // Remove trailing comments: "SELECT a -- like this"
                let mut stripped = line.to_string();
                let offset = stripped.find("--").unwrap_or(stripped.len());
                stripped.replace_range(offset.., "");
                stripped.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        let stripped = block_comment.replace_all(&sql, "");

        if !stripped.ends_with(";") {
            format!("{stripped};")
        } else {
            stripped.to_string()
        }
    }

    pub fn sql(&self) -> &str {
        &self.0
    }

    /// Add another query to the end of this one.
    pub fn append(&mut self, other: Self) -> TernResult<()> {
        let mut buf = String::new();
        writeln!(buf, "{}", self.0)?;
        writeln!(buf, "{}", other.0)?;
        self.0 = buf;
        Ok(())
    }

    /// Split a query comprised of multiple statements.
    ///
    /// For queries having `no_tx = true`, a migration comprised of multiple,
    /// separate SQL statements needs to be broken up so that the statements can
    /// run sequentially.  Otherwise, many backends will run the collection of
    /// statements in a transaction automatically, which breaches the `no_tx`
    /// contract.
    ///
    /// _Warning_: This is sensitive to the particular character sequence for
    /// writing comments.  Only `--` and C-style `/* ... */` are treated
    /// correctly because this is valid comment syntax in any of the supported
    /// backends.  A line starting with `#`, for instance, will not be treated as
    /// a comment, and so only in MySQL where that does denote a comment, the
    /// function may not separate multiple statements correctly, possibly leading
    /// to syntax errors during query execution.
    pub fn split_statements(&self) -> TernResult<Vec<String>> {
        let mut statements = Vec::new();
        self.sanitize()
            .lines()
            .try_fold(String::new(), |mut buf, line| {
                if line.trim().is_empty() {
                    return Ok(buf);
                }
                writeln!(buf, "{line}")?;
                // If the line ends with `;` this is the end of the statement, so
                // push the accumulated buffer to the vector and start a new one.
                if line.ends_with(";") {
                    statements.push(buf);
                    Ok::<String, std::fmt::Error>(String::new())
                } else {
                    Ok(buf)
                }
            })?;

        Ok(statements)
    }
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Name/version derived from the migration source filename.
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct MigrationId {
    /// Version parsed from the migration filename.
    version: i64,
    /// Description parsed from the migration filename.
    description: String,
}

impl MigrationId {
    pub fn new(version: i64, description: String) -> Self {
        Self {
            version,
            description,
        }
    }

    pub fn version(&self) -> i64 {
        self.version
    }

    pub fn description(&self) -> String {
        self.description.clone()
    }
}

impl std::fmt::Display for MigrationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "V{}__{}", self.version, self.description)
    }
}

impl From<AppliedMigration> for MigrationId {
    fn from(value: AppliedMigration) -> Self {
        Self {
            version: value.version,
            description: value.description,
        }
    }
}

/// An `AppliedMigration` is the information about a migration that completed
/// successfully and it is also a row in the schema history table.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct AppliedMigration {
    /// The migration version.
    pub version: i64,
    /// The description of the migration.
    pub description: String,
    /// The contents of the migration file at the time it was applied.
    pub content: String,
    /// How long the migration took to run in milliseconds.
    pub duration_ms: i64,
    /// The timestamp of when the migration was applied.
    pub applied_at: DateTime<Utc>,
}

impl AppliedMigration {
    pub fn new(
        id: MigrationId,
        content: &str,
        duration_ms: i64,
        applied_at: DateTime<Utc>,
    ) -> Self {
        Self {
            version: id.version,
            description: id.description,
            content: content.into(),
            duration_ms,
            applied_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Query;

    const SQL_IN1: &str = "
-- This is a comment.
SELECT
  *
FROM
  the_schema.the_table
WHERE
  everything = 'is_good'
";
    const SQL_OUT1: &str = "SELECT
  *
FROM
  the_schema.the_table
WHERE
  everything = 'is_good';
";
    const SQL_IN2: &str = "
-- tern:noTransaction
SELECT count(e.*),
  e.x,
  e.y -- This is the column called `y`
FROM /* A comment block can even be like this */ the_table
  as e
JOIN another USING (id)
/*
This is a multi
line
comment
*/
WHERE false;

SELECT a
from x
-- Asdfsdfsdfsdfsdsdf /* Unnecessary comment */
where false

;
";
    const SQL_OUT21: &str = "SELECT count(e.*),
  e.x,
  e.y
FROM  the_table
  as e
JOIN another USING (id)
WHERE false;
";

    const SQL_OUT22: &str = "SELECT a
from x
where false
;
";

    #[test]
    fn split_one() {
        let q1 = Query::new(SQL_IN1.to_string());
        let res1 = q1.split_statements();
        assert!(res1.is_ok());
        let split1 = res1.unwrap();
        assert_eq!(split1, vec![SQL_OUT1.to_string()]);
    }

    #[test]
    fn split_two() {
        let q2 = Query::new(SQL_IN2.to_string());
        let res2 = q2.split_statements();
        assert!(res2.is_ok());
        let split2 = res2.unwrap();
        assert_eq!(split2, vec![SQL_OUT21.to_string(), SQL_OUT22.to_string()]);
    }
}
