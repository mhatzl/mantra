use clap::Parser;

#[tokio::main]
async fn main() {
    let cfg = mantra::cfg::Config::parse();

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_target(false)
        .init();

    if let Err(err) = mantra::run(cfg).await {
        println!("{err}");
        std::process::exit(-1);
    }
}
