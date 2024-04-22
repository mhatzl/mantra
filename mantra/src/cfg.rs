use crate::{
    cmd::Cmd,
    db::{self, ProjectOrigin},
};

#[derive(clap::Parser)]
pub struct Config {
    #[command(flatten)]
    pub db: db::Config,
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(clap::Args)]
pub struct ProjectConfig {
    pub name: String,
    #[command(subcommand)]
    pub origin: ProjectOrigin,
}

#[derive(clap::Args)]
pub struct DeprecateConfig {
    pub project_name: String,
    #[arg(alias = "id")]
    pub req_ids: Vec<String>,
}

#[derive(clap::Args)]
pub struct UntraceableConfig {
    pub project_name: String,
    #[arg(alias = "id")]
    pub req_ids: Vec<String>,
}

#[derive(clap::Args)]
pub struct DeleteReqsConfig {
    pub ids: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteTracesConfig {
    pub req_ids: Option<Vec<String>>,
    pub projects: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteCoverageConfig {
    pub projects: Option<Vec<String>>,
    pub tests: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteProjectConfig {
    pub projects: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteDeprecatedConfig {
    pub req_ids: Option<Vec<String>>,
    pub projects: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteUntraceableConfig {
    pub req_ids: Option<Vec<String>>,
    pub projects: Option<Vec<String>>,
}
