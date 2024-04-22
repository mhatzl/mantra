use clap::Parser;

#[tokio::main]
async fn main() {
    let cfg = mantra::cfg::Config::parse();

    mantra::run(cfg).await;
}
