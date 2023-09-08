use crate::cli::Cli;
use clap::Parser;
use logid::{
    log_id::LogLevel,
    logging::filter::{AddonFilter, FilterConfigBuilder},
};

mod check;
mod cli;
mod globals;
mod references;
mod status;
mod sync;
mod wiki;

fn main() {
    let cli = Cli::parse();

    let _ = logid::logging::filter::set_filter(
        FilterConfigBuilder::new(LogLevel::Info)
            .allowed_addons(AddonFilter::Infos)
            .build(),
    );

    let log_handler = logid::event_handler::builder::LogEventHandlerBuilder::new()
        .to_stderr()
        .all_log_events()
        .build()
        .expect("Could not setup logging.");

    let start = std::time::Instant::now();

    let cmd_result = cli.run_cmd().or_else(|err| {
        logid::log!(err);
        Ok::<(), cli::CmdError>(())
    });

    let end = std::time::Instant::now();

    println!(
        "Took: {}ms",
        end.checked_duration_since(start).unwrap().as_millis()
    );

    if cmd_result.is_err() {
        log_handler.shutdown();
        std::process::exit(1);
    }
}
