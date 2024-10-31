use clap::Parser;
use derrick::cli::Opt;
use derrick::prelude::*;
use derrick::Runner;

use pg_envvar_query::{ExampleMigrate, GetEnvVar};

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenvy::dotenv().ok();

    if let Err(e) = derrick::cli::run::<ExampleMigrate, ()>(Opt::parse(), ()).await {
        println!("error: {e}");

        std::process::exit(1);
    }
}
