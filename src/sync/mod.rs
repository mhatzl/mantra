use std::path::PathBuf;

/// Parameters for the `sync` command.
///
/// [req:sync]
pub struct SyncParameter {
    /// The name of the branch the project currently is in.
    pub branch_name: String,

    /// The folder that is searched recursively for requirement references.
    ///
    /// [req:sync]
    pub proj_folder: PathBuf,

    /// The folder that is searched recursively for defined requirements.
    ///
    /// [req:sync], [req:wiki]
    pub req_folder: PathBuf,

    /// The prefix every wiki-link must have to correctly point to the requirement inside the wiki.
    ///
    /// [req:sync], [req:wiki]
    pub wiki_url_prefix: String,
}

pub fn sync(params: SyncParameter) -> Result<(), SyncError> {
    todo!()
}

/// Possible errors that may occure during synchronisation.
pub enum SyncError {}
