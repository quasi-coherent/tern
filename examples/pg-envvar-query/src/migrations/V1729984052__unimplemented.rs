use derrick::Error;
use derrick::QueryBuilder;

use crate::ExampleMigrate;

#[derive(QueryBuilder)]
#[migration(no_transaction, runtime = ExampleMigrate)]
pub struct Unimplemented;

pub async fn build_query(runtime: &mut ExampleMigrate) -> Result<String, Error> {
    let user = runtime
        .env
        .get_var("USER")
        .expect("could not get `USER` from environment");
    let sql = format!("INSERT INTO dmd_test(x, y) VALUES ({}, '{}');", 25, user);
    println!("sql: {sql}");

    Ok(sql)
}
