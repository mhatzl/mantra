use clap::Parser;

#[tokio::main]
async fn main() {
    let cfg = mantra::cfg::Config::parse();

    if let Err(err) = mantra::run(cfg).await {
        println!("{err}");
        std::process::exit(-1);
    }
}
