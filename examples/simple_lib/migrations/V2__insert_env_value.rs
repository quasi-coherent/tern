//! # V2__insert_env_value
//!
//! This module implements the migration interface for a type we define.
//!
//! `Migration` can be implemented directly, but a likely more common approach
//! is to instead implement [`tern::migration::ResolveQuery`], which implies the
//! `Migration` impl.  `ResolveQuery` is more convenient to write; it needs only
//! to provide a context, an init method, and a query.
use std::fmt::Write as _;
use std::str::FromStr as _;
use tern::sqlx::SqlxError;
use tern::{Migration, Query, ResolveQuery, TernResult};

use super::SimpleMigrate;

/// Either requires `ResolveQuery`, or remove the derive and implement
/// `Migration` by hand.
#[derive(Migration)]
pub struct InsertSimpleExampleUser {
    user: String,
    n: u8,
}

impl ResolveQuery for InsertSimpleExampleUser {
    type Ctx = SimpleMigrate;

    async fn init(ctx: &mut Self::Ctx) -> TernResult<Self> {
        let user = ctx.env.get_var("USER")?;
        let n = ctx.env.get_from_str::<u8>("V2_REPEAT_N").unwrap_or(1);
        Ok(Self { user, n })
    }

    async fn resolve(&self, ctx: &mut Self::Ctx) -> TernResult<Query> {
        let maxx = ctx.get_max_x().await?;
        let user = &self.user;
        let n = self.n;

        // `Query` has a builder interface to make this easier. Here,
        // `Query::builder()` makes a query that is sent as one single statement.
        // The builder returned by `Query::sequential_builder()`, in contrast,
        // can be used to define a query that runs a sequence of statements
        // without a transaction.
        //
        // Building one INSERT with multiple VALUES is possible, but not by
        // using query builder methods, as these expect a complete statement.
        // Instead, we'd need to assemble the `VALUES (x1, y1), (x2, y2)...`
        // first and _then_ push to the builder.
        //
        // Not doing that because it's not the point.
        let builder = (1..=n).try_fold(Query::builder(), |mut acc, i| {
            acc.push_sql(format!(
                "INSERT INTO simple_example(x, y) VALUES ({i}, '{user}');"
            ))?;

            Ok::<_, tern::TernError>(acc)
        })?;

        Ok(builder.build())
    }
}
