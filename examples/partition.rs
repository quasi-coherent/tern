// TODO(tern-derive rework): re-enable this example once `PartitionExample`
// derives `TernApp` again. `TernCli::try_init_with` requires that impl, so the
// body is stubbed for the 4.0 compile-fix pass. Original body preserved below.
pub mod partition_lib;

fn main() {
    eprintln!(
        "the `partition` example is disabled during the 4.0 compile-fix pass"
    );
}

// use simplelog::{Config, LevelFilter, SimpleLogger};
// use tern::TernCli;
// use partition_lib::PartitionExample;
//
// #[tokio::main]
// async fn main() {
// let _ = SimpleLogger::init(LevelFilter::Debug, Config::default());
//
// let app = TernCli::try_init_with(PartitionExample::new)
// .await
// .expect("Could not construct CLI");
//
// match app.run().await {
// Ok(complete) => println!("{complete}"),
// Err(e) => {
// println!("{e}");
// std::process::exit(1);
// },
// }
// }
