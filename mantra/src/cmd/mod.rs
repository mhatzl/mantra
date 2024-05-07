use crate::cfg::{DeleteCoverageConfig, DeleteReqsConfig, DeleteReviewConfig, DeleteTracesConfig};

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
    DeleteCoverage(DeleteCoverageConfig),
    DeleteReview(DeleteReviewConfig),
}
