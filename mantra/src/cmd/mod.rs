use crate::cfg::ProjectConfig;

pub mod analyze;
pub mod coverage;
pub mod extract;
pub mod report;
pub mod trace;

#[derive(clap::Subcommand)]
pub enum Cmd {
    Trace(trace::Config),
    Extract(extract::Config),
    AddProject(ProjectConfig),
}
