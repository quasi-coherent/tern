use tern::error::{DatabaseError as _, TernResult};
use tern::{ContextOptions, SqlxPgExecutor};
use tern::{MigrationContext, MigrationSource};

#[derive(MigrationContext, MigrationSource)]
#[tern(source = "src/migrations")]
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
