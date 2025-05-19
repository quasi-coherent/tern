use index_partitioned_table_concurrently::PgContextOptions;
use tern::App;

#[tokio::main]
async fn main() {
    let mut env = env_logger::Builder::from_default_env();
    env.filter(None, log::LevelFilter::Info).init();

    let app = App::new(PgContextOptions);

    match app.run().await {
        Err(e) => log::error!("{e}"),
        Ok(Some(report)) => report
            .iter_results()
            .for_each(|result| log::info!("{result}")),
        Ok(_) => log::info!("OK"),
    }
}
