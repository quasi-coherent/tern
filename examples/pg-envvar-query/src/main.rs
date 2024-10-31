use derrick::prelude::*;
use derrick::sqlx_postgres::SqlxPgMigrate;
use derrick::types::HistoryTableInfo;
use derrick::Runner;

use pg_envvar_query::{ExampleMigrate, GetEnvVar};

#[tokio::main]
async fn main() {
    println!("asdf")
    // env_logger::init();

    // let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not found");
    // let mut runtime = ExampleMigrate::initialize(db_url, GetEnvVar)
    //     .await
    //     .expect("could not create migration runtime");
    // let history_table = HistoryTableInfo::default();

    // let (runner, ready_await) = migrations::init();

    // println!("");
    // ready
    //     .run(runtime)
    //     .await
    //     .expect("could not apply migrations");
}
