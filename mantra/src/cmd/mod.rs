use crate::cfg::MantraConfigPath;

use self::report::ReportCliConfig;

pub mod analyze;
pub mod coverage;
pub mod report;
pub mod requirements;
pub mod review;
pub mod trace;

const REVIEW_DATE_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day] [hour]:[minute][optional [:[second][optional [.[subsecond]]]]]"
);

time::serde::format_description!(review_date_format, PrimitiveDateTime, REVIEW_DATE_FORMAT);

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Cmd {
    Report(Box<ReportCliConfig>),
    Collect(MantraConfigPath),
    /// Delete test runs and reviews that have no linked requirement or coverage remaining.
    Prune,
    /// Delete all collected date in the database.
    Clear,
}
