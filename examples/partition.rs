use simplelog::{Config, LevelFilter, SimpleLogger};
use tern::TernCli;

pub mod partition_lib;
use partition_lib::PartitionExample;

#[tokio::main]
async fn main() {
    let _ = SimpleLogger::init(LevelFilter::Debug, Config::default());

    let app = TernCli::try_init_with(PartitionExample::new)
        .await
        .expect("Could not construct CLI");

    match app.run().await {
        Ok(complete) => println!("{complete}"),
        Err(e) => {
            println!("{e}");
            std::process::exit(1);
        },
    }
}
