//! # V2__insert_env_value
//!
//! This module implements the migration interface for a type we define.
//!
//! `Migration` can be implemented directly, but a likely more common approach
//! is to instead implement [`tern::migration::ResolveQuery`], which implies the
//! `Migration` impl.  `ResolveQuery` is more convenient to write; it needs only
//! to provide a context, an init method, and a query.
//!
//! TODO(tern-derive rework): disabled for the 4.0 compile-fix pass. The
//! `Migration` derive + `ResolveMigration` impl below use the pre-refactor API.
//! Relocations for re-enabling:
//!   `tern::Query`           -> `tern::query::Query`
//!   `tern::ResolveMigration` -> `tern::migration::ResolveQuery`
//!   `ResolveMigration::resolve` -> `ResolveQuery::resolve_query`
//! Not compiled while the parent `#[derive(TernApp)]` is disabled.
// use tern::error::{TernError, TernResult};
// use tern::{Migration, Query, ResolveMigration};
//
// use super::SimpleExample;
//
// Needs `ResolveMigration`.
// #[derive(Migration)]
// pub struct SimpleExampleInsertUser {
// user: String,
// }
//
// impl ResolveMigration for SimpleExampleInsertUser {
// type Ctx = SimpleExample;
//
// async fn init(ctx: &mut Self::Ctx) -> TernResult<Self> {
// let user = ctx.env.get_var("USER")?;
// Sanitizing inputs:
// if !user.chars().all(|c| c.is_alphabetic()) {
// return Err(TernError::Invalid(format!(
// "non-alpha chars in USER: {user}"
// )));
// }
//
// Ok(Self { user })
// }
//
// async fn resolve(&self, ctx: &mut Self::Ctx) -> TernResult<Query> {
// let user = &self.user;
// let maxx = ctx.get_max_x().await? + 1;
// let range = maxx..=maxx + 10;
//
// `Query` has a builder interface to make this easier. Here,
// `Query::builder()` makes a query that is sent as one single
// statement. `Query::builder().with_notx()` will run individual
// statements sequentially outside of a transaction.
//
// Building one INSERT with multiple VALUES is possible, but not by
// using query builder methods, as these expect a complete statement.
// Instead, we'd need to assemble the `VALUES (x1, y1), (x2, y2)...`
// first and _then_ push to the builder.
//
// Not doing that because it's not the point.
// range
// .fold(Query::builder(), |mut acc, i| {
// acc.push_sql(format!(
// "INSERT INTO simple_example(x, y) VALUES ({i}, '{user}');"
// ));
// acc
// })
// .build()
// .read_query()
// }
// }
