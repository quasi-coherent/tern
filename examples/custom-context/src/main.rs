use custom_context::ExampleOptions;
use tern::cli::App;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let mut env = env_logger::Builder::from_default_env();
    env.filter(None, log::LevelFilter::Info).init();

    let app = App::new(ExampleOptions);

    if let Err(e) = app.run().await {
        log::error!("{e}");
        std::process::exit(1);
    }
}
