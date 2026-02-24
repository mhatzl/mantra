// use crate::cfg::MantraConfigPath;

// use self::report_old::ReportCliConfig;

pub mod analyze;
pub mod collect;
pub mod report;

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Cmd {
    Report,
    Collect,
    /// Delete test runs and reviews that have no linked requirement or coverage remaining.
    Prune,
    /// Delete all collected date in the database.
    Clear,
}
