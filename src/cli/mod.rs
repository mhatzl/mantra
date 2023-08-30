//! Contains the cli struct, and invocation points to all supported commands.

use clap::{Parser, Subcommand};

use crate::sync::SyncParameter;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    pub fn run_cmd(&self) -> Result<(), CmdError> {
        match &self.command {
            Some(cmd) => cmd.run(),
            None => Err(CmdError::MissingCmd),
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Synchronizes references between wiki and project.
    #[command(name = "sync")]
    Sync {
        /// Parameters for synchronization.
        #[command(flatten)]
        param: SyncParameter,
    },
}

impl Command {
    fn run(&self) -> Result<(), CmdError> {
        match self {
            Command::Sync { param } => crate::sync::sync(param).map_err(|err| CmdError::SyncError),
        }
    }
}

#[derive(Debug, thiserror::Error, logid::ErrLogId)]
pub enum CmdError {
    #[error("Synchronization between wiki and project failed.")]
    SyncError,

    #[error("No command was given.")]
    MissingCmd,
}
