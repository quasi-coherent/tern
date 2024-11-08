use clap::Parser;
use derrick::cli::Opt;
use pg_envvar_query::ExampleMigrate;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let mut env = env_logger::Builder::from_default_env();
    env.filter(None, log::LevelFilter::Info).init();

    if let Err(e) = derrick::cli::run::<ExampleMigrate, ()>(Opt::parse(), ()).await {
        println!("error: {e}");

        std::process::exit(1);
    }
}
