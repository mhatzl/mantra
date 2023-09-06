//! Contains global parameter that are needed for all commands.

use std::path::PathBuf;

use clap::Args;

/// Parameters needed for all commands.
#[derive(Args, Debug, Clone)]
pub struct GlobalParameter {
    /// The folder that is searched recursively for defined requirements.
    ///
    /// [req:wiki]
    #[arg(index = 1, required = true)]
    pub req_folder: PathBuf,

    /// The folder that is searched recursively for requirement references.
    /// If not set, the current folder is used.
    #[arg(index = 2, required = false, default_value = "./")]
    pub proj_folder: PathBuf,

    /// The prefix every wiki-link must have to correctly point to the requirement inside the wiki.
    /// This option is required to validate wiki-links that may be set for references.
    ///
    /// [req:wiki]
    #[arg(long = "wiki-url-prefix")]
    pub wiki_url_prefix: Option<String>,
}
