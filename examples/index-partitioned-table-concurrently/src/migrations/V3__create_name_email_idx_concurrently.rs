use super::{Partition, PgMigrationContext};

use std::fmt::Write;
use tern::Migration;
use tern::error::TernResult;
use tern::source::{Query, QueryBuilder};

const PARENT_IDX_NAME: &str = "example_partitioned_name_email_dx";

/// This migration can't run in a transaction because that would defeat the
/// purpose of creating an index concurrently if it was even allowed, thus
/// the `no_transaction`.
#[derive(Migration)]
#[tern(no_transaction)]
pub struct TernMigration;
impl TernMigration {
    /// This is a "metadata-only" operation.  We have to do it first in order to
    /// attach the child partition indices along the way.
    fn create_on_only_parent(&self) -> String {
        format!(
            "
CREATE INDEX IF NOT EXISTS {PARENT_IDX_NAME} ON ONLY example.partitioned (name, email);
"
        )
    }

    /// The template for creating the child index concurrently and attaching it
    /// to the parent index.
    fn create_child_idx(&self, partition: &Partition) -> TernResult<String> {
        let idx_name = partition.idx_name(PARENT_IDX_NAME);
        let sql = format!(
            "
CREATE INDEX CONCURRENTLY IF NOT EXISTS {idx_name} ON {partition} (name, email);
ALTER INDEX example.{PARENT_IDX_NAME} ATTACH PARTITION example.{idx_name};
",
        );

        Ok(sql)
    }
}

impl QueryBuilder for TernMigration {
    /// Building the query for this specific context.
    type Ctx = PgMigrationContext;

    async fn build(&self, ctx: &mut PgMigrationContext) -> TernResult<Query> {
        let sql = ctx.get_partitions().await?.into_iter().try_fold(
            self.create_on_only_parent(),
            |mut buf, partition| {
                let create_idx_query = self.create_child_idx(&partition)?;
                writeln!(buf, "{create_idx_query}")?;
                Ok::<String, tern::error::Error>(buf)
            },
        )?;
        let query = Query::new(sql);

        Ok(query)
    }
}
