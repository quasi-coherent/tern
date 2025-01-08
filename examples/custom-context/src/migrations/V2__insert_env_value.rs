//! A Rust migration has two simple requirements:
//!
//! * A unit struct `TernMigration` that derives `Migration`.  It can
//!   optionally have the attribute `tern(no_transaction)`, which means
//!   that the query will not be ran in a transaction.
//! * The struct `TernMigration` needs to implement the trait `QueryBuilder`,
//!   which tells the runtime what to do to construct the query it needs to run
//!   the actual migration.
//!
//! The associated `Ctx` in the query builder is whatever you've defined the
//! migration context to be.  It's the thing deriving `MigrationContext`.  In
//! this example, it has the database connection type and `GetEnvVar`.  The
//! connection type is of course required, but anything additional is up to the
//! user according to the needs of the migration.  For this example, `GetEnvVar`
//! is just a type that can get an environment variable.
//!
//! The query for this migration depends on the current time and place; it needs
//! whatever the $USER is and the maximum value of the `z` column in `dmd_test`.
//! So the query has to be built dynamically at the time when the migration is
//! applied, which is what `QueryBuilder` exists for.
use tern::error::{TernResult, Error};
use tern::migration::{Query, QueryBuilder};
use tern::Migration;

use super::ExampleContext;

#[derive(Migration)]
#[tern(no_transaction)]
pub struct TernMigration;

impl QueryBuilder for TernMigration {
    type Ctx = ExampleContext;

    async fn build(&self, ctx: &mut Self::Ctx) -> TernResult<Query> {
        let user = ctx
            .env
            .get_var("USER")
            .expect("could not get `USER` from environment");
        let max_value = ctx
            .max_value()
            .await
            .map_err(|e| Error::ResolveQuery(format!("{e:?}")))?;
        let sql = format!(
            "
INSERT INTO dmd_test(x, y)
  VALUES ({max_value}, '{user}');
"
        );
        let query = Query::new(sql);

        Ok(query)
    }
}
