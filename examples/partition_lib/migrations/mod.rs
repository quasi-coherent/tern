use std::fmt::{self, Display, Formatter};
use tern::executor::sqlx::{SqlxError, SqlxPgExecutor};
use tern::executor::util::sqlx::{FromRow, query_as};
use tern::executor::{ConnOpt, ExecutorOptions};
use tern::{ContextOptions, TernApp, TernResult};

/// The `TernApp` for our partitioned table example.
#[derive(Clone, Debug, TernApp)]
#[tern(
    source = "examples/partition_lib/migrations",
    table = "_partition_history"
)]
pub struct PartitionExample {
    #[tern(executor_via)]
    pub exec: SqlxPgExecutor,
}

impl PartitionExample {
    /// Query system tables to get the list of DB objects currently inheriting
    /// from the target partitioned one.
    pub async fn get_child_partitions(&self) -> TernResult<Vec<Partition>> {
        let partitions: Vec<Partition> = query_as(
            "
SELECT
  b.relnamespace::regnamespace::text AS schema,
  b.relname AS child_table
FROM
  pg_catalog.pg_inherits a
  JOIN pg_catalog.pg_class b
  ON a.inhrelid = b.oid
WHERE
  inhparent = 'examples.partition_example'::regclass
",
        )
        .fetch_all(self.exec.inner())
        .await
        .map_err(SqlxError::from)?;
        Ok(partitions)
    }
}

impl ContextOptions<PartitionExample> for ConnOpt {
    async fn initialize(self) -> TernResult<PartitionExample> {
        let exec: SqlxPgExecutor = self.connect().await?;
        Ok(PartitionExample { exec })
    }
}

/// A record from the result of the query `get_partitions`.
#[derive(FromRow)]
pub struct Partition {
    schema: String,
    child_table: String,
}

impl Partition {
    /// Parent index qualified by the child partition name.
    pub fn idx_name(&self, parent_idx: &str) -> String {
        format!("{}_{parent_idx}", self.child_table)
    }
}

impl Display for Partition {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.schema, self.child_table)
    }
}
