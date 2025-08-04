use index_partitioned_table_concurrently::PgContextOptions;
use tern::Tern;

#[tokio::main]
async fn main() {
    env_logger::init();

    match Tern::run_cli(PgContextOptions).await {
        Ok(Some(report)) => report
            .iter_results()
            .for_each(|result| log::info!("{result}")),
        Ok(_) => log::info!("OK"),
        Err(e) => log::error!("{e}"),
    }
}
