use std::fmt::{self, Display, Formatter};
use tern::TernResult;
use tern::exec::sqlx::{self, FromRow};
use tern::exec::{ConnStr, SqlxError, SqlxPgExecutor};
// TODO(tern-derive rework): re-add `use tern::TernApp;` when the derive below
// is re-enabled.

/// The `TernApp` for our partitioned table example.
// TODO(tern-derive rework): re-enable the `TernApp` derive (and the `#[tern]`
// attrs, including `#[tern(executor_via)]` on `exec` below) once the macro
// emits the new core API.
#[derive(Debug)]
// #[derive(Debug, TernApp)]
// #[tern(
//     source = "examples/partition_lib/migrations",
//     table = "_partition_history"
// )]
pub struct PartitionExample {
    // #[tern(executor_via)]
    pub exec: SqlxPgExecutor,
}

impl PartitionExample {
    /// Initialize a value from a connection string.
    pub async fn new(conn: ConnStr) -> TernResult<Self> {
        let exec = SqlxPgExecutor::new(&conn).await?;
        Ok(Self { exec })
    }

    /// Query system tables to get the list of DB objects currently inheriting
    /// from the target partitioned one.
    pub async fn get_child_partitions(&mut self) -> TernResult<Vec<Partition>> {
        let partitions: Vec<Partition> = sqlx::query_as(
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
        .fetch_all(self.exec.inner_mut())
        .await
        .map_err(SqlxError::from)?;
        Ok(partitions)
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
