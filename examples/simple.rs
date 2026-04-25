use tern::Tern;

pub mod simple_lib;
use simple_lib::SimpleExample;

#[tokio::main]
async fn main() {
    match Tern::run_new(SimpleExample::init).await {
        Ok(completed) => println!("{completed}"),
        Err(e) => {
            println!("{e}");
            std::process::exit(1);
        },
    }
}
