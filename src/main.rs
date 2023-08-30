use std::path::PathBuf;

use clap::Parser;
use logid::{
    log_id::LogLevel,
    logging::filter::{AddonFilter, FilterConfigBuilder},
};
use references::ReferencesMap;
use wiki::Wiki;

use crate::cli::Cli;

mod cli;
mod references;
mod req;
mod sync;
mod wiki;

fn main() {
    let _ = logid::logging::filter::set_filter(
        FilterConfigBuilder::new(LogLevel::Info)
            .allowed_addons(AddonFilter::Infos)
            .build(),
    );

    let _log_handler = logid::event_handler::builder::LogEventHandlerBuilder::new()
        .to_stderr()
        .all_log_events()
        .build()
        .expect("Could not setup logging.");

    let cli = Cli::parse();

    let start = std::time::Instant::now();

    // let wiki = Wiki::try_from(PathBuf::from("../../evident-wiki/5-Requirements")).unwrap();
    // let ref_map = ReferencesMap::try_from((&wiki, PathBuf::from("../../evident"))).unwrap();

    // let _ = sync::sync(sync::SyncParameter {
    //     branch_name: "main".to_string(),
    //     proj_folder: PathBuf::from("../../evident"),
    //     req_folder: PathBuf::from("../../evident-wiki/5-Requirements"),
    //     wiki_url_prefix: None,
    // });

    let _ = cli.run_cmd().or_else(|err| {
        logid::log!(err);
        Ok::<(), cli::CmdError>(())
    });

    let end = std::time::Instant::now();

    // dbg!(wiki.sub_reqs(&format!("subs")));
    // dbg!(wiki);
    // dbg!(ref_map);

    println!(
        "Took: {}ms",
        end.checked_duration_since(start).unwrap().as_millis()
    );
}
