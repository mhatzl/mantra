use crate::{
    cmd::Cmd,
    db::{self},
};

#[derive(clap::Parser)]
pub struct Config {
    #[command(flatten)]
    pub db: db::Config,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeleteReqsConfig {
    #[arg(long)]
    pub ids: Option<Vec<String>>,
    /// Delete requirements before the set generation.
    #[arg(long)]
    pub before: Option<i64>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeleteTracesConfig {
    #[arg(long, alias = "id")]
    pub req_ids: Option<Vec<String>>,
    /// Delete traces before the set generation.
    #[arg(long)]
    pub before: Option<i64>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeleteTestRunsConfig {
    #[arg(long, alias = "older-than")]
    pub before: Option<String>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeleteCoverageConfig {
    #[arg(long, alias = "id")]
    pub req_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeleteReviewsConfig {
    #[arg(long, alias = "older-than")]
    pub before: Option<String>,
}
