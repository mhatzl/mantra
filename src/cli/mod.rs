//! Contains the cli struct, and invocation points to all supported commands.

use clap::{Parser, Subcommand};

use crate::{check::CheckParameter, status::StatusParameter, sync::SyncParameter};

const HELP_TEMPLATE: &str = r#"
{before-help}{name} {version} - {about-with-newline}
Created by: {author-with-newline}
{usage-heading} {usage}

{all-args}{after-help}"#;

#[derive(Parser)]
#[command(help_template = HELP_TEMPLATE, author, version, about, long_about = None)]
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
    ///
    /// [req:sync]
    #[command(name = "sync")]
    Sync {
        /// Parameters for synchronization.
        #[command(flatten)]
        param: SyncParameter,
    },

    /// Checks wiki structure and references in the project.
    ///
    /// [req:check]
    #[command(name = "check")]
    Check {
        /// Parameters for validation of wiki and references.
        #[command(flatten)]
        param: CheckParameter,
    },

    /// Creates status overview of the wiki.
    ///
    /// [req:status]
    #[command(name = "status")]
    Status {
        /// Parameters for the status command.
        #[command(flatten)]
        param: StatusParameter,
    },
}

impl Command {
    fn run(&self) -> Result<(), CmdError> {
        match self {
            Command::Sync { param } => crate::sync::sync(param).map_err(|_| CmdError::SyncError),
            Command::Check { param } => {
                crate::check::check(param).map_err(|_| CmdError::CheckError)
            }
            Command::Status { param } => {
                crate::status::status(param).map_err(|_| CmdError::StatusError)
            }
        }
    }
}

#[derive(Debug, thiserror::Error, logid::ErrLogId)]
pub enum CmdError {
    #[error("Synchronization between wiki and project failed.")]
    SyncError,

    #[error("Validation of wiki and/or references in the project failed.")]
    CheckError,

    #[error("Creating status overview for wiki failed.")]
    StatusError,

    #[error("No command was given. Use '-h' or '--help' for help.")]
    MissingCmd,
}
