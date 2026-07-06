use tern::TernCli;

pub mod simple_lib;
use simple_lib::SimpleExample;

#[tokio::main]
async fn main() {
    match TernCli::try_init_with(SimpleExample::new)
        .await
        .expect("could not create app")
        .run()
        .await
    {
        Ok(completed) => println!("{completed}"),
        Err(e) => {
            println!("{e}");
            std::process::exit(1);
        },
    }
}
