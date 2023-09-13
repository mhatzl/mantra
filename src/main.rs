//! Command line tool for [mantra](https://github.com/mhatzl/mantra).
//! See the [requirements section](https://github.com/mhatzl/mantra/wiki/5-Requirements)
//! in the [wiki](https://github.com/mhatzl/mantra/wiki) for more info on how to manage requirements with *mantra*.
//!
//! **Available commands:**
//!
//! - `check`
//!
//!   ```text
//!   Checks wiki structure and references in the project.
//!   
//!   [req:check]
//!   
//!   Usage: mantra check [OPTIONS] <REQ_FOLDER> [PROJ_FOLDER]
//!   
//!   Arguments:
//!     <REQ_FOLDER>
//!             The folder that is searched recursively for defined requirements.
//!   
//!             [req:wiki]
//!   
//!     [PROJ_FOLDER]
//!             The folder that is searched recursively for requirement references. If not set, the current folder is used
//!   
//!             [default: ./]
//!   
//!   Options:
//!         --branch-name <BRANCH_NAME>
//!             The name of the branch project references should be validated against in the wiki. If not set, 'main' is used as default branch.
//!   
//!             [req:wiki.ref_list]
//!   
//!             [default: main]
//!   
//!         --repo-name <REPO_NAME>
//!             Optional repository name in case multiple repositories point to the same wiki.
//!   
//!             [req:wiki.ref_list.repo]
//!   
//!     -h, --help
//!             Print help (see a summary with '-h')
//!   ```
//!
//! - `release`
//!
//!   ```text
//!   Creates a release report.
//!   
//!   [req:release]
//!   
//!   Usage: mantra release [OPTIONS] --release-tag <RELEASE_TAG> <REQ_FOLDER>
//!   
//!   Arguments:
//!     <REQ_FOLDER>
//!             The folder that is searched recursively for defined requirements.
//!   
//!             [req:wiki]
//!   
//!   Options:
//!         --branch <BRANCH>
//!             The branch name to create the overview for. Is used as first branch for comparisons
//!   
//!             [default: main]
//!   
//!         --release-tag <RELEASE_TAG>
//!             The tag/version for this release
//!   
//!         --wiki-url-prefix <WIKI_URL_PREFIX>
//!             An optional URL prefix for links to the wiki. This options must be set if links to the wiki should be added in the report.
//!   
//!             [req:release]
//!   
//!         --report-file <REPORT_FILE>
//!             An optional filepath for the report. If this option is not set, the report is printed to stdout.
//!   
//!             **Note:** The report will be a markdown file, so the given extension is ignored.
//!   
//!         --checklist
//!             Set this flag to turn this report into a checklist for requirements tagged with *manual*.
//!   
//!             [req:release.checklist]
//!   
//!         --repo-name <REPO_NAME>
//!             Optional repository name in case multiple repositories point to the same wiki.
//!   
//!             [req:wiki.ref_list.repo]
//!   
//!     -h, --help
//!             Print help (see a summary with '-h')
//!   ```
//!
//! - `status`
//!
//!   ```text
//!   Creates status overview of the wiki.
//!   
//!   [req:status]
//!   
//!   Usage: mantra status [OPTIONS] <REQ_FOLDER>
//!   
//!   Arguments:
//!     <REQ_FOLDER>
//!             The folder that is searched recursively for defined requirements.
//!   
//!             [req:wiki]
//!   
//!   Options:
//!         --branch <BRANCH>
//!             The branch name to create the overview for. Is used as first branch for comparisons.
//!   
//!             [req:status.branch], [req:status.cmp]
//!   
//!             [default: main]
//!   
//!         --repo-name <REPO_NAME>
//!             Optional repository name for the `branch` option in case multiple repositories point to the same wiki.
//!   
//!             [req:wiki.ref_list.repo]
//!   
//!         --cmp-branch <CMP_BRANCH>
//!             An optional branch to compare against the branch set with `--branch`.
//!   
//!             [req:status.cmp]
//!   
//!         --cmp-repo-name <CMP_REPO_NAME>
//!             Optional repository name for the `cmp-branch` option in case multiple repositories point to the same wiki.
//!   
//!             [req:wiki.ref_list.repo]
//!   
//!         --detail-ready
//!             Flag to output detailed information about *ready* requirements.
//!   
//!             [req:status.branch]
//!   
//!         --detail-active
//!             Flag to output detailed information about *active* requirements.
//!   
//!             [req:status.branch]
//!   
//!         --detail-deprecated
//!             Flag to output detailed information about *deprecated* requirements.
//!   
//!             [req:status.branch]
//!   
//!         --detail-manual
//!             Flag to output detailed information about requirements flagged to require *manual* verification.
//!   
//!             [req:status.branch]
//!   
//!     -h, --help
//!             Print help (see a summary with '-h')
//!   ```
//!
//! - `sync`
//!
//!   ```text
//!   Synchronizes references between wiki and project.
//!   
//!   [req:sync]
//!   
//!   Usage: mantra.exe sync [OPTIONS] <REQ_FOLDER> [PROJ_FOLDER]
//!   
//!   Arguments:
//!     <REQ_FOLDER>
//!             The folder that is searched recursively for defined requirements.
//!   
//!             [req:wiki]
//!   
//!     [PROJ_FOLDER]
//!             The folder that is searched recursively for requirement references. If not set, the current folder is used
//!   
//!             [default: ./]
//!   
//!   Options:
//!         --branch-name <BRANCH_NAME>
//!             The name of the branch project references should be synchronized to in the wiki. If not set, 'main' is used as default branch.
//!   
//!             [req:wiki.ref_list]
//!   
//!             [default: main]
//!   
//!         --branch-link <BRANCH_LINK>
//!             Optional link to the branch.
//!   
//!             [req:wiki.ref_list.branch_link]
//!   
//!         --repo-name <REPO_NAME>
//!             Optional repository name in case multiple repositories point to the same wiki.
//!   
//!             [req:wiki.ref_list.repo]
//!   
//!     -h, --help
//!             Print help (see a summary with '-h')
//!   ```

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
mod release;
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
