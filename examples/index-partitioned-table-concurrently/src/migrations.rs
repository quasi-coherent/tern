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

/// To be able to use this with the CLI.
pub struct PgContextOptions;

impl ContextOptions for PgContextOptions {
    type Ctx = PgMigrationContext;

    async fn connect(&self, db_url: &str) -> TernResult<PgMigrationContext> {
        PgMigrationContext::new(db_url).await
    }
}

/// A row in the system table `pg_inherits` that we'll query to get the current
/// list of child partitions of the partitioned table.
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
