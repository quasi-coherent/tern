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
use crate::error::{DatabaseError as _, Error, TernResult};

use chrono::{DateTime, Utc};
use futures_core::{Future, future::BoxFuture};
use regex::Regex;
use sql_splitter::parser::{Parser, SqlDialect};
use std::fmt::Write;
use std::io::Error as IoError;
use std::sync::OnceLock;
use std::time::Instant;

/// The context in which a migration run occurs.
pub trait MigrationContext
where
    Self: MigrationSource<Ctx = Self> + Send + Sync + 'static,
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
            let applied =
                migration.to_applied(duration_ms, applied_at, query.sql());
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
    fn previously_applied(
        &mut self,
    ) -> BoxFuture<'_, TernResult<Vec<AppliedMigration>>> {
        Box::pin(self.executor().get_all_applied(Self::HISTORY_TABLE))
    }

    /// Check that the history table exists and create it if not.
    fn check_history_table(&mut self) -> BoxFuture<'_, TernResult<()>> {
        Box::pin(
            self.executor().create_history_if_not_exists(Self::HISTORY_TABLE),
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
    fn apply_tx(
        &mut self,
        query: &Query,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Apply the `Query` for the migration _not_ in a transaction.
    fn apply_no_tx(
        &mut self,
        query: &Query,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// `CREATE IF NOT EXISTS` the history table.
    fn create_history_if_not_exists(
        &mut self,
        history_table: &str,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// `DROP` the history table.
    fn drop_history(
        &mut self,
        history_table: &str,
    ) -> impl Future<Output = TernResult<()>> + Send;

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
    fn insert_into_history_query(
        history_table: &str,
        applied: &AppliedMigration,
    ) -> Query;

    /// The query to return all rows from the schema history table.
    fn select_star_from_history_query(history_table: &str) -> Query;

    /// Query to insert or update a record in the history table.
    fn upsert_history_query(
        history_table: &str,
        applied: &AppliedMigration,
    ) -> Query;
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
    fn build(
        &self,
        ctx: &mut Self::Ctx,
    ) -> impl Future<Output = TernResult<Query>> + Send;
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
    fn build<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>>;

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
        AppliedMigration::new(
            self.migration_id(),
            content,
            duration_ms,
            applied_at,
        )
    }
}

/// A type that is used to collect a [`MigrationSet`] -- migrations that are not
/// applied yet -- which is used as the input to runner commands.
pub trait MigrationSource {
    /// A context that the set of migrations returned by `migration_set` would
    /// need in order to be applied.
    type Ctx: MigrationContext;

    /// The set of migrations since `last_applied`.
    fn migration_set(
        &self,
        last_applied: Option<i64>,
    ) -> MigrationSet<Self::Ctx>;
}

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
        self.migrations.iter().map(|m| m.version()).collect::<Vec<_>>()
    }

    /// The version/name of migrations in this migration set.
    pub fn migration_ids(&self) -> Vec<MigrationId> {
        self.migrations.iter().map(|m| m.migration_id()).collect::<Vec<_>>()
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
        Self { version, description }
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
        Self { version: value.version, description: value.description }
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

fn dialect_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r".*tern:noTransaction,?([a-z]*)").unwrap())
}

/// A SQL query.
#[derive(Debug, Clone)]
pub struct Query(pub(crate) String);

impl Query {
    /// New `Query` from a string.
    pub fn new(sql: String) -> Self {
        Self(sql)
    }

    /// Return the underlying query text.
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

    /// Split a query comprised of multiple statements into a collection of
    /// the atomic, single statements.
    ///
    /// This is necessary to honor the contract of "tern:noTransaction", since
    /// sending a query with multiple statements is treated as one prepared
    /// statement and ran in a transaction automatically.
    pub fn split_statements(&self) -> TernResult<Vec<String>> {
        let sql = self.0.as_bytes();
        let dialect = self.detect_dialect().unwrap_or(SqlDialect::Postgres);

        let mut parser = Parser::with_dialect(sql, sql.len(), dialect);
        let mut stats = Vec::new();

        while let Some(stat_bytes) =
            parser.read_statement().map_err(Error::split_err(stats.len()))?
        {
            let raw = String::from_utf8(stat_bytes)
                .map_err(IoError::other)
                .map_err(Error::split_err(stats.len()))?;

            // Drop "queries" that are only whitespace, like a trailing newline
            // at the end of the file, or newlines between queries.
            let stat = raw.trim();
            if !stat.is_empty() {
                stats.push(stat.to_string());
            }
        }

        Ok(stats)
    }

    fn detect_dialect(&self) -> Option<SqlDialect> {
        let mut first = self.0.lines().take(1);
        let l = first.next()?;
        let re = dialect_re();
        let caps = re.captures(l)?;
        Some(match caps.get(1)?.as_str() {
            "sqlite" => SqlDialect::Sqlite,
            "mysql" => SqlDialect::MySql,
            "postgres" => SqlDialect::Postgres,
            _ => return None,
        })
    }
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dialect() {
        let x = "-- tern:noTransaction";
        let y = "-- tern:noTransaction,sqlite";
        let z = "-- Comment with tern:noTransaction,postgres in the middle";
        let t = "-- tern:noTransaction,dynamodb";
        let xx = Query::new(x.into());
        let yy = Query::new(y.into());
        let zz = Query::new(z.into());
        let tt = Query::new(t.into());
        assert_eq!(xx.detect_dialect(), None);
        assert_eq!(yy.detect_dialect(), Some(SqlDialect::Sqlite));
        assert_eq!(zz.detect_dialect(), Some(SqlDialect::Postgres));
        assert_eq!(tt.detect_dialect(), None);
    }

    #[test]
    fn handles_single() {
        const SQL: &str = "
SELECT
 blah,
 whatever, -- this column is way totally
 region_id,
 prod_discount_region_start_date_early,
 prod_discount_region_start_date,
 prod_discount_region_end_date
FROM
 prod_discount_date
WHERE
 whatever = 'way;totally' -- Way; very totally
 AND region_id = 5;
";
        let query = Query::new(SQL.into());
        let res = query.split_statements();
        assert!(res.is_ok());
        let mut stats = res.unwrap();
        assert_eq!(stats.len(), 1);
        let stat = stats.pop().unwrap();
        assert_eq!(&stat, SQL.trim());
    }

    #[test]
    fn handles_multiple() {
        const SQL: &str = r#"
-- tern:noTransaction,postgres
SELECT
  column1 AS "asdf;lkh",
  column2
FROM
  the_table as a
/* Why
would anyone do this;
it's absurd
*/
JOIN
  the_other_table as b
USING (column3);

SELECT * INTO the_table_recent
FROM the_table
WHERE
  column1 != 'string--with--special/*characters*/and--terminator;'
  AND recent = true;
"#;
        let query = Query::new(SQL.into());
        let res = query.split_statements();
        assert!(res.is_ok_and(|ss| ss.len() == 2));
    }
}
