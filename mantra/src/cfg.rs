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
