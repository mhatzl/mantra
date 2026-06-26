// use crate::cfg::MantraConfigPath;

// use self::report_old::ReportCliConfig;

use crate::cmd::{collect::cfg::CollectArguments, report::cfg::ReportArguments};

pub mod analyze;
pub mod collect;
pub mod report;

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Cmd {
    Report(ReportArguments),
    Collect(CollectArguments),
    /// Delete test runs and reviews that have no linked requirement or coverage remaining.
    Prune,
    /// Delete all collected date in the database.
    Clear,
}
