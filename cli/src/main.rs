use clap::Parser;

#[tokio::main]
async fn main() {
    let cfg = mantra::Config::parse();

    mantra::run(cfg).await;
}
