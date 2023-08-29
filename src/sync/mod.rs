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

    for (filepath, changed_req) in changes.ordered_file_changes() {
        let mut lines: Vec<String> = std::fs::read_to_string(filepath)
            .map_err(|_| SyncError::AccessingWikiFile(filepath.clone()))?
            .lines()
            .map(|s| s.to_string())
            .collect();

        // Note: We assume that the requirement is still at the correct line as retrieved by the Wiki struct.
        for req in changed_req {
            let mut ref_list_line_nr = req.line_nr + 1; // Note: +1 to start looking for *reference* list entries after heading and blank line.

            // Had no *references* list before
            if wiki
                .req(&req.head.id)
                .unwrap_or_else(|| panic!("Changed requirement '{}' not in wiki.", &req.head.id))
                .ref_list
                .is_empty()
            {
                lines.insert(ref_list_line_nr, "**References:**".to_string());
                ref_list_line_nr += 1;
                lines.insert(ref_list_line_nr, "".to_string());
                ref_list_line_nr += 1;
            } else {
                // Jump to first entry
                while !lines
                    .get(ref_list_line_nr)
                    .unwrap_or(&String::from(""))
                    .starts_with('-')
                {
                    ref_list_line_nr += 1;
                }
            }

            for entry in req.ref_list {
                let mut extend_list = false;

                match lines.get_mut(ref_list_line_nr) {
                    Some(entry_line) if entry_line.starts_with('-') => {
                        let _ = std::mem::replace(entry_line, entry.to_string());
                        ref_list_line_nr += 1;
                    }
                    Some(_) | None => {
                        extend_list = true;
                    }
                }

                if extend_list {
                    if ref_list_line_nr >= lines.len() {
                        lines.push(entry.to_string());
                    } else {
                        lines.insert(ref_list_line_nr, entry.to_string());
                    }

                    ref_list_line_nr += 1;
                }
            }
        }

        let mut content = lines.join("\n");
        content.push('\n');
        std::fs::write(filepath, content)
            .map_err(|_| SyncError::AccessingWikiFile(filepath.clone()))?;
    }

    Ok(())
}

/// Possible errors that may occure during synchronisation.
pub enum SyncError {
    WikiSetup,
    ReferenceCounting,
    AccessingWikiFile(PathBuf),
}

impl From<WikiError> for SyncError {
    fn from(_value: WikiError) -> Self {
        SyncError::WikiSetup
    }
}

impl From<ReferencesMapError> for SyncError {
    fn from(_value: ReferencesMapError) -> Self {
        SyncError::ReferenceCounting
    }
}
