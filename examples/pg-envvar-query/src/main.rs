use derrick::prelude::*;
use derrick::sqlx_postgres::SqlxPgMigrate;
use derrick::types::HistoryTableInfo;
use derrick::Runner;

use pg_envvar_query::{migrations, ExampleMigrate, GetEnvVar};

#[tokio::main]
async fn main() {
    env_logger::init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not found");
    let migrate = SqlxPgMigrate::connect(db_url)
        .await
        .expect("could not create connection");
    let mut runtime = ExampleMigrate {
        migrate,
        env: GetEnvVar,
    };
    let history_table = HistoryTableInfo::default();
    let runner = Runner::new(history_table);

    let ready = migrations::ready(runner, &mut runtime)
        .await
        .expect("could not prepare final migration set");

    ready
        .run(runtime)
        .await
        .expect("could not apply migrations");
}
