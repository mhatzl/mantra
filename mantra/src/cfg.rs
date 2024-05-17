use crate::{
    cmd::Cmd,
    db::{self},
};

#[derive(clap::Parser)]
pub struct Config {
    #[command(flatten)]
    pub db: db::Config,

    #[arg(long)]
    pub project_name: Option<String>,
    #[arg(long)]
    pub project_version: Option<String>,
    #[arg(long)]
    pub project_link: Option<String>,
    #[arg(long)]
    pub cargo: bool,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub name: String,
    pub full_version: String,
    pub major_version: Option<usize>,
    pub link: Option<String>,
}

#[derive(clap::Args)]
pub struct DeleteReqsConfig {
    #[arg(long)]
    pub ids: Option<Vec<String>>,
    /// Delete requirements before the set generation.
    #[arg(long)]
    pub before: Option<i64>,
}

#[derive(clap::Args)]
pub struct DeleteTracesConfig {
    #[arg(long, alias = "id")]
    pub req_ids: Option<Vec<String>>,
    /// Delete traces before the set generation.
    #[arg(long)]
    pub before: Option<i64>,
}

#[derive(clap::Args)]
pub struct DeleteTestRunsConfig {
    #[arg(long, alias = "older-than")]
    pub before: Option<String>,
}

#[derive(clap::Args)]
pub struct DeleteCoverageConfig {
    #[arg(long, alias = "id")]
    pub req_ids: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteReviewsConfig {
    #[arg(long, alias = "older-than")]
    pub before: Option<String>,
}
