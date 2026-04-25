use simplelog::{Config, LevelFilter, SimpleLogger};
use tern::Tern;
use tern::executor::ConnOpt;

pub mod partition_lib;
use partition_lib::PartitionExample;

#[tokio::main]
async fn main() {
    let _ = SimpleLogger::init(LevelFilter::Debug, Config::default());

    match Tern::<PartitionExample>::run_options::<ConnOpt>().await {
        Ok(complete) => println!("{complete}"),
        Err(e) => {
            println!("{e}");
            std::process::exit(1);
        },
    }
}
