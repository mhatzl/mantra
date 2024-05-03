use crate::cfg::{
    DeleteCoverageConfig, DeleteDeprecatedConfig, DeleteManualRequirementsConfig, DeleteReqsConfig,
    DeleteReviewConfig, DeleteTracesConfig, DeprecateConfig, ManualRequirementConfig,
};

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
    DeprecateReq(DeprecateConfig),
    AddManuelReq(ManualRequirementConfig),
    DeleteReqs(DeleteReqsConfig),
    DeleteTraces(DeleteTracesConfig),
    DeleteCoverage(DeleteCoverageConfig),
    DeleteDeprecated(DeleteDeprecatedConfig),
    DeleteManualReq(DeleteManualRequirementsConfig),
    DeleteReview(DeleteReviewConfig),
}
