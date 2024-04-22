use crate::cfg::{
    DeleteCoverageConfig, DeleteDeprecatedConfig, DeleteProjectConfig, DeleteReqsConfig,
    DeleteTracesConfig, DeleteUntraceableConfig, DeprecateConfig, ProjectConfig, UntraceableConfig,
};

pub mod analyze;
pub mod coverage;
pub mod extract;
pub mod report;
pub mod trace;

#[derive(clap::Subcommand)]
pub enum Cmd {
    Trace(trace::Config),
    Extract(extract::Config),
    Coverage(coverage::CliConfig),
    AddProject(ProjectConfig),
    DeprecateReq(DeprecateConfig),
    AddUntraceable(UntraceableConfig),
    DeleteReqs(DeleteReqsConfig),
    DeleteTraces(DeleteTracesConfig),
    DeleteCoverage(DeleteCoverageConfig),
    DeleteProjects(DeleteProjectConfig),
    DeleteDeprecated(DeleteDeprecatedConfig),
    DeleteUntraceable(DeleteUntraceableConfig),
}
