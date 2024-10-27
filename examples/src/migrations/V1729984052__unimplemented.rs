use derrick::prelude::*;
use derrick::Error;
use derrick::QueryBuilder;

use crate::ExampleMigrate;

#[derive(QueryBuilder)]
#[migration(no_transaction, runtime = ExampleMigrate)]
pub struct Unimplemented;

pub async fn build_query(runtime: &mut ExampleMigrate) -> Result<String, Error> {
    let random = runtime
        .env
        .get_var("RANDOM")
        .expect("could not get `RANDOM` from environment")
        .parse::<i32>()
        .expect("could not parse `RANDOM` into `i32`");
    let sql = format!(
        "INSERT INTO dmd_test(x, y) VALUES ({}, '{}');",
        random,
        "random value".to_string()
    );

    Ok(sql)
}
