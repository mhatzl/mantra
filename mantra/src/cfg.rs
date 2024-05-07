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

#[derive(clap::Args)]
pub struct DeleteReqsConfig {
    #[arg(long)]
    pub ids: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteTracesConfig {
    #[arg(long, alias = "id")]
    pub req_ids: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteTestRunConfig {
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub date: Option<String>,
}

#[derive(clap::Args)]
pub struct DeleteCoverageConfig {
    #[arg(long, alias = "id")]
    pub req_ids: Option<Vec<String>>,
}

#[derive(clap::Args)]
pub struct DeleteReviewConfig {
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub date: Option<String>,
}
