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
    #[arg(long)]
    pub name: String,
    #[command(subcommand)]
    pub origin: ProjectOrigin,
}

#[derive(clap::Args)]
pub struct DeprecateConfig {
    #[arg(long)]
    pub project_name: String,
    #[arg(long, alias = "id")]
    pub req_ids: Vec<String>,
}

#[derive(clap::Args)]
pub struct UntraceableConfig {
    #[arg(long)]
    pub project_name: String,
    #[arg(long, alias = "id")]
    pub req_ids: Vec<String>,
}

#[derive(clap::Args)]
pub struct DeleteReqsConfig {
    #[arg(long)]
    pub ids: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteTracesConfig {
    #[arg(long)]
    pub req_ids: Option<Vec<String>>,
    #[arg(long)]
    pub projects: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteCoverageConfig {
    #[arg(long)]
    pub projects: Option<Vec<String>>,
    #[arg(long)]
    pub tests: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteProjectConfig {
    #[arg(long)]
    pub projects: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteDeprecatedConfig {
    #[arg(long)]
    pub req_ids: Option<Vec<String>>,
    #[arg(long)]
    pub projects: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteUntraceableConfig {
    #[arg(long)]
    pub req_ids: Option<Vec<String>>,
    #[arg(long)]
    pub projects: Option<Vec<String>>,
}
