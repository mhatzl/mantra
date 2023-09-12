//! Contains functionality for the `status` command.
//!
//! [req:status]

use std::path::PathBuf;

use clap::Args;

use crate::wiki::{Wiki, WikiError};

mod branch;
mod cmp;

/// Parameters for the `status` command.
///
/// [req:status]
#[derive(Args, Debug, Clone)]
pub struct StatusParameter {
    /// The folder that is searched recursively for defined requirements.
    ///
    /// [req:wiki]
    #[arg(index = 1, required = true)]
    pub req_folder: PathBuf,

    /// The branch name to create the overview for.
    /// Is used as first branch for comparisons.
    ///
    /// [req:status.branch], [req:status.cmp]
    #[arg(long, alias = "branch-name", required = false, default_value = "main")]
    pub branch: String,

    /// Optional repository name for the `branch` option in case multiple repositories point to the same wiki.
    ///
    /// [req:wiki.ref_list.repo]
    #[arg(long, alias = "repo")]
    pub repo_name: Option<String>,

    /// An optional branch to compare against the branch set with `--branch`.
    ///
    /// [req:status.cmp]
    #[arg(long)]
    pub cmp_branch: Option<String>,

    /// Optional repository name for the `cmp-branch` option in case multiple repositories point to the same wiki.
    ///
    /// [req:wiki.ref_list.repo]
    #[arg(long, alias = "cmp-repo")]
    pub cmp_repo_name: Option<String>,

    /// Flag to output detailed information about *ready* requirements.
    ///
    /// [req:status.branch]
    #[arg(long)]
    pub detail_ready: bool,

    /// Flag to output detailed information about *active* requirements.
    ///
    /// [req:status.branch]
    #[arg(long)]
    pub detail_active: bool,

    /// Flag to output detailed information about *deprecated* requirements.
    ///
    /// [req:status.branch]
    #[arg(long)]
    pub detail_deprecated: bool,

    /// Flag to output detailed information about requirements flagged
    /// to require *manual* verification.
    ///
    /// [req:status.branch]
    #[arg(long)]
    pub detail_manual: bool,
}

/// Command to output an overview for the wiki.
///
/// [req:status]
pub fn status(param: &StatusParameter) -> Result<(), StatusError> {
    let wiki = Wiki::try_from(&param.req_folder)?;

    let overview = match &param.cmp_branch {
        Some(branch_b) => self::cmp::status_cmp(&wiki, &param.branch, branch_b),
        None => self::branch::status_branch(&wiki, param),
    };

    println!("{overview}");

    Ok(())
}

/// Possible errors that may occure while creating a status overview.
#[derive(Debug, thiserror::Error, logid::ErrLogId)]
pub enum StatusError {
    #[error("Failed to parse the wiki.")]
    WikiSetup,
}

impl From<WikiError> for StatusError {
    fn from(_value: WikiError) -> Self {
        StatusError::WikiSetup
    }
}
