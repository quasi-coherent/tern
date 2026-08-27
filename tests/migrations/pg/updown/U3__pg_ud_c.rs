//! Dynamic up migration in an up/down pair; the down side is plain SQL.
//!
//! TODO(tern-derive rework): disabled for the 4.0 compile-fix pass. The
//! `Migration` derive + `ResolveMigration` impl below use the pre-refactor API.
//! Relocations for re-enabling:
//!   `tern::Query`           -> `tern::query::Query`
//!   `tern::ResolveMigration` -> `tern::migration::ResolveQuery`
//!   `ResolveMigration::resolve` -> `ResolveQuery::resolve_query`
//! Not compiled while the parent `#[derive(TernApp)]` is disabled.
// use tern::error::TernResult;
// use tern::{Migration, Query, ResolveMigration};
//
// use super::PgUpDown;
//
// #[derive(Migration)]
// pub struct PgUpDownCreateC {
// rows: i64,
// }
//
// impl ResolveMigration for PgUpDownCreateC {
// type Ctx = PgUpDown;
//
// async fn init(_ctx: &mut Self::Ctx) -> TernResult<Self> {
// Ok(Self { rows: 2 })
// }
//
// async fn resolve(&self, _ctx: &mut Self::Ctx) -> TernResult<Query> {
// let mut builder = Query::builder();
// builder.push_sql("CREATE TABLE tern_pg_ud_c (x bigint);");
// for i in 1..=self.rows {
// builder.push_sql(format!("INSERT INTO tern_pg_ud_c VALUES ({i});"));
// }
// builder.build().read_query()
// }
// }
