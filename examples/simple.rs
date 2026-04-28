use tern::App;

pub mod simple;
use simple::ExampleOptions;

#[tokio::main]
async fn main() {
    let mut env = env_logger::Builder::from_default_env();
    env.filter(None, log::LevelFilter::Info).init();

    let app = App::new(ExampleOptions);

    if let Err(e) = app.run().await {
        log::error!("{e}");
        std::process::exit(1);
    }
}
