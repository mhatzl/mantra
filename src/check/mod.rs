//! Contains functionality to validate the wiki, and references to requirements in the project.
//!
//! [req:check]

use clap::Args;

use crate::globals::GlobalParameter;

/// Parameters for the `check` command.
///
/// [req:check]
#[derive(Args, Debug, Clone)]
pub struct CheckParameter {
    /// Global parameter needed for all commands.
    #[command(flatten)]
    pub global: GlobalParameter,

    /// The name of the branch project references should be validated against in the wiki.
    /// If not set, 'main' is used as default branch.
    ///
    /// [req:wiki.ref_list]
    #[arg(long, required = false, default_value = "main")]
    pub branch_name: String,
}
