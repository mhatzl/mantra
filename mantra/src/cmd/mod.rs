use crate::cfg::{DeleteReqsConfig, DeleteReviewsConfig, DeleteTestRunsConfig, DeleteTracesConfig};

pub mod analyze;
pub mod coverage;
pub mod extract;
pub mod report;
pub mod review;
pub mod trace;

#[derive(clap::Subcommand)]
pub enum Cmd {
    Trace(trace::Config),
    Extract(extract::Config),
    Coverage(coverage::CliConfig),
    DeleteReqs(DeleteReqsConfig),
    DeleteTraces(DeleteTracesConfig),
    DeleteTestRuns(DeleteTestRunsConfig),
    DeleteReviews(DeleteReviewsConfig),
    Clean,
}
