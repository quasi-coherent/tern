//! # index-partitioned-table-concurrently
//!
//! A standard index build in postgres locks out writes on the table until it is
//! done, so to avoid this interruption of service, an index can be created with the
//! keyword `CONCURRENTLY`.  For tables that are [partitioned][partitioned-table],
//! however, using this keyword is an error.  From the [documentation][pg-index-docs]:
//!
//! > Concurrent builds for indexes on partitioned tables are currently not supported.
//! > However, you may concurrently build the index on each partition individually
//! > and then finally create the partitioned index non-concurrently in order to
//! > reduce the time where writes to the partitioned table will be locked out.
//! > In this case, building the partitioned index is a metadata only operation.
//!
//! The issue with this approach, however, is that it is not known _a priori_ what
//! the names of the individual partitions are, and moreover, these partitions could
//! be created and detached periodically, so the collection of partitions to index
//! is different at any given time.
//!
//! This example demonstrates an approach to address these difficulties.
//!
//! [partitioned-table]: https://www.postgresql.org/docs/current/ddl-partitioning.html
//! [pg-index-docs]: https://www.postgresql.org/docs/current/sql-createindex.html#SQL-CREATEINDEX-CONCURRENTLY
use tern::error::{DatabaseError as _, TernResult};
use tern::{ContextOptions, SqlxPgExecutor};
use tern::{MigrationContext, MigrationSource};

#[derive(MigrationContext, MigrationSource)]
#[tern(source = "migrations")]
pub struct PgMigrationContext {
    #[tern(executor_via)]
    executor: SqlxPgExecutor,
}

impl PgMigrationContext {
    pub async fn new(db_url: &str) -> TernResult<Self> {
        let executor = SqlxPgExecutor::new(db_url).await?;
        Ok(Self { executor })
    }

    /// The index build migration needs the child partitions of the table.
    /// So the context needs to be able to obtain them.
    /// This function defines the context's ability to fetch the current list of
    /// child partitions for the partitioned table at the time the migration
    /// query needs to be built.  This is done by querying two system tables
    /// storing inheritance relations between tables.
    pub async fn get_partitions(&self) -> TernResult<Vec<Partition>> {
        let partitions: Vec<Partition> = sqlx::query_as(
            "
SELECT
  b.relnamespace::regnamespace::text AS relnamespace,
  b.relname AS relname
FROM
  pg_catalog.pg_inherits a
JOIN
  pg_catalog.pg_class b
ON a.inhrelid = b.oid
WHERE
  inhparent = 'example.partitioned'::regclass
",
        )
        .fetch_all(&self.executor.pool())
        .await
        .tern_result()?;

        Ok(partitions)
    }
}

/// To be able to use this with the CLI, it needs to know how to build a generic
/// migration context given a connection string, so `ContextOptions` does this.
pub struct PgContextOptions;

impl ContextOptions for PgContextOptions {
    type Ctx = PgMigrationContext;

    async fn connect(&self, db_url: &str) -> TernResult<PgMigrationContext> {
        PgMigrationContext::new(db_url).await
    }
}

/// A record from the result of the query `get_partitions`.
#[derive(sqlx::FromRow)]
pub struct Partition {
    relnamespace: String,
    relname: String,
}

impl Partition {
    /// Parent index qualified by the child partition name.
    pub fn idx_name(&self, parent_idx: &str) -> String {
        format!("{}_{parent_idx}", self.relname)
    }

    pub fn schema(&self) -> &str {
        &self.relnamespace
    }

    pub fn table(&self) -> &str {
        &self.relname
    }
}

impl std::fmt::Display for Partition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.relnamespace, self.relname)
    }
}
