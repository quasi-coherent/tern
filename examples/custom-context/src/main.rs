use custom_context::ExampleOptions;
use tern::Tern;

#[tokio::main]
async fn main() {
    env_logger::init();

    if let Err(e) = Tern::run_cli::<ExampleOptions>().await {
        log::error!("{e}");
        std::process::exit(1);
    }
}
