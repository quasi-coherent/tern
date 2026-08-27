//! TODO(tern-derive rework): disabled for the 4.0 compile-fix pass. The
//! `Migration` derive + `ResolveMigration` impl below use the pre-refactor API.
//! Relocations for re-enabling:
//!   `tern::Query`           -> `tern::query::Query`
//!   `tern::ResolveMigration` -> `tern::migration::ResolveQuery`
//!   `ResolveMigration::resolve` -> `ResolveQuery::resolve_query`
//! Not compiled while the parent `#[derive(TernApp)]` is disabled.
// use tern::{Migration, Query, ResolveMigration, TernResult};
//
// use super::{Partition, PartitionExample};
//
// const PARENT_IDX_NAME: &str = "part_eg_name_idx";
//
// #[derive(Migration)]
// pub struct CreateNameIdx;
//
// impl CreateNameIdx {
// fn only_parent_idx(&self) -> String {
// format!(
// "
// CREATE INDEX IF NOT EXISTS {PARENT_IDX_NAME}
// ON ONLY examples.partition_example (name);
// "
// )
// }
//
// fn child_idx(&self, child: &Partition) -> String {
// let idx_name = child.idx_name(PARENT_IDX_NAME);
// format!(
// "
// CREATE INDEX CONCURRENTLY IF NOT EXISTS {idx_name} ON {child} (name);
// ALTER INDEX examples.{PARENT_IDX_NAME} ATTACH PARTITION examples.{idx_name};
// ",
// )
// }
// }
//
// impl ResolveMigration for CreateNameIdx {
// type Ctx = PartitionExample;
//
// async fn init(_: &mut Self::Ctx) -> TernResult<Self> {
// Ok(Self)
// }
//
// async fn resolve(&self, ctx: &mut Self::Ctx) -> TernResult<Query> {
// Get the present list of child partitions.
// let partitions = ctx.get_child_partitions().await?;
// let mut builder = Query::builder();
// builder.push_sql(self.only_parent_idx());
//
// partitions
// .iter()
// .fold(builder, |mut acc, partition| {
// acc.push_sql(self.child_idx(partition));
// acc
// })
// .build()
// .with_notx() // Return a query that does not run in a transaction
// .read_query()
// }
// }
