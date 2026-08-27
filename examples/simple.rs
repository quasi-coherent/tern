// TODO(tern-derive rework): re-enable this example once `SimpleExample` derives
// `TernApp` again. `TernCli::try_init_with` requires that impl, so the body is
// stubbed for the 4.0 compile-fix pass. Original body preserved below.
pub mod simple_lib;

fn main() {
    eprintln!(
        "the `simple` example is disabled during the 4.0 compile-fix pass"
    );
}

// use tern::TernCli;
// use simple_lib::SimpleExample;
//
// #[tokio::main]
// async fn main() {
// match TernCli::try_init_with(SimpleExample::new)
// .await
// .expect("could not create app")
// .run()
// .await
// {
// Ok(completed) => println!("{completed}"),
// Err(e) => {
// println!("{e}");
// std::process::exit(1);
// },
// }
// }
