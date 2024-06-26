use crate::cfg::{
    CollectConfig, DeleteOldConfig, DeleteReqsConfig, DeleteReviewsConfig, DeleteTestRunsConfig,
    DeleteTracesConfig,
};

use self::report::ReportConfig;

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
    #[command(subcommand)]
    Trace(trace::TraceKind),
    #[command(subcommand)]
    Requirements(requirements::Format),
    Coverage(coverage::Config),
    /// Delete requirements and traces that have not been added or updated
    /// with the latest `extract` or `trace` command.
    DeleteOld(DeleteOldConfig),
    DeleteReqs(DeleteReqsConfig),
    DeleteTraces(DeleteTracesConfig),
    DeleteTestRuns(DeleteTestRunsConfig),
    DeleteReviews(DeleteReviewsConfig),
    Review(review::ReviewConfig),
    Report(ReportConfig),
    Collect(CollectConfig),
    /// Delete test runs and reviews that have no linked requirement or coverage remaining.
    Clean,
}
