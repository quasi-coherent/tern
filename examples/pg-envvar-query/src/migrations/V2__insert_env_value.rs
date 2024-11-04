//! Two requirements of a Rust migration:
//!
//! * There's a struct that derives `QueryBuilder` and has at least the argument
//!   `runtime = ...` for the `migration` attribute.  The struct doesn't matter;
//!   there just has to be _something_ to put the derive macro on.  The type that
//!   that `runtime = ...` does matter though: it has to be the same as the main
//!   "app" type -- the type having the dataase connection and implementing the
//!   `Migrate` trait.
//!      - The `migration` attribute takes an optional argument `no_transaction`,
//!        which instructs the migration runner to apply this migration outside
//!        of a databasetransaction.
//! * A function as below called `build_query` that takes the runtime and
//!   returns the SQL query for the migration.
use derrick::Error;
use derrick::QueryBuilder;

use super::ExampleMigrate;

#[derive(QueryBuilder)]
#[migration(no_transaction, runtime = ExampleMigrate)]
pub struct InsertValueFromEnv;

pub async fn build_query(runtime: &mut ExampleMigrate) -> Result<String, Error> {
    let user = runtime
        .env
        .get_var("USER")
        .expect("could not get `USER` from environment");
    let sql = format!("INSERT INTO dmd_test(x, y) VALUES ({}, '{}');", 25, user);

    Ok(sql)
}
