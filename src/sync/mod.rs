use std::path::PathBuf;

use crate::{
    references::{changes::ReferenceChanges, ReferencesMap, ReferencesMapError},
    wiki::{Wiki, WikiError},
};

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
    let wiki = Wiki::try_from(params.req_folder)?;
    let ref_map = ReferencesMap::try_from((&wiki, params.proj_folder))?;

    let changes = ReferenceChanges::new(params.branch_name, &wiki, &ref_map);

    dbg!(changes);

    Ok(())
}

/// Possible errors that may occure during synchronisation.
pub enum SyncError {
    CouldNotSetupWiki,
    CouldNotCountReferences,
}

impl From<WikiError> for SyncError {
    fn from(_value: WikiError) -> Self {
        SyncError::CouldNotSetupWiki
    }
}

impl From<ReferencesMapError> for SyncError {
    fn from(_value: ReferencesMapError) -> Self {
        SyncError::CouldNotCountReferences
    }
}
