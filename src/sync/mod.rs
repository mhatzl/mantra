//! Contains functionality to synchronize the requirements found in a wiki, with references to those requirements in a project.
//!
//! [req:sync]

use std::path::PathBuf;

use clap::Args;

use crate::{
    globals::GlobalParameter,
    references::{changes::ReferenceChanges, ReferencesError, ReferencesMap},
    wiki::{ref_list::ProjectLine, Wiki, WikiError},
};

/// Parameters for the `sync` command.
///
/// [req:sync]
#[derive(Args, Debug, Clone)]
pub struct SyncParameter {
    /// Global parameter needed for all commands.
    #[command(flatten)]
    pub global: GlobalParameter,

    /// The name of the branch project references should be synchronized to in the wiki.
    /// If not set, 'main' is used as default branch.
    ///
    /// [req:wiki.ref_list]
    #[arg(long, required = false, default_value = "main")]
    pub branch_name: String,

    /// Optional link to the branch.
    ///
    /// [req:wiki.ref_list.branch_link]
    #[arg(long)]
    pub branch_link: Option<String>,

    /// Optional repository name in case multiple repositories point to the same wiki.
    ///
    /// [req:wiki.ref_list.repo]
    #[arg(long, alias = "repo")]
    pub repo_name: Option<String>,
}

/// Synchronizes requirement references between requirements in a wiki, and references to them in a project.
///
/// [req:sync]
pub fn sync(params: &SyncParameter) -> Result<(), SyncError> {
    let wiki = Wiki::try_from(&params.global.req_folder)?;
    let ref_map = ReferencesMap::try_from((&wiki, &params.global.proj_folder))?;

    let changes = ReferenceChanges::new(
        ProjectLine::new(
            params.repo_name.clone(),
            params.branch_name.clone(),
            params.branch_link.clone(),
        ),
        &wiki,
        &ref_map,
    )?;
    let ordered_file_changes = changes.ordered_file_changes();

    if ordered_file_changes.is_empty() {
        logid::log!(SyncInfo::Unchanged, "Wiki and project already in-sync.");
        return Ok(());
    }

    for (filepath, changed_req) in ordered_file_changes {
        let orig_content_res = std::fs::read_to_string(filepath)
            .map_err(|_| logid::pipe!(SyncError::AccessingWikiFile(filepath.clone())));

        let orig_content = match orig_content_res {
            Ok(orig) => orig,
            Err(err) => {
                if crate::globals::early_exit() {
                    return Err(err);
                } else {
                    continue;
                }
            }
        };

        let orig_lines: Vec<&str> = orig_content.lines().collect();
        let mut orig_line_nr = 0;
        let mut new_lines: Vec<String> = Vec::with_capacity(orig_lines.len());

        // Note: We assume that the requirement is still at the correct line as retrieved by the Wiki struct.
        for req in changed_req {
            // Note: To start looking for *reference* list entries after heading and blank line.
            while orig_line_nr <= req.line_nr
                || orig_lines
                    .get(orig_line_nr)
                    .map_or_else(|| false, |line| !line.trim().is_empty())
            {
                match orig_lines.get(orig_line_nr) {
                    Some(orig_line) => new_lines.push(orig_line.to_string()),
                    // Note: Might be the case if a requirement has no content besides the heading line.
                    None => new_lines.push("".to_string()),
                }
                orig_line_nr += 1;
            }

            let untraced_before = wiki
                .req(&req.head.id)
                .unwrap_or_else(|| {
                    unreachable!("Changed requirement '{}' not in wiki.", &req.head.id)
                })
                .ref_list
                .is_empty();

            if untraced_before {
                new_lines.push("".to_string());
                new_lines.push("**References:**".to_string());
                new_lines.push("".to_string());
            } else {
                // Jump to first entry
                while !orig_lines.get(orig_line_nr).unwrap_or(&"").starts_with('-') {
                    new_lines.push(orig_lines.get(orig_line_nr).unwrap_or(&"").to_string());
                    orig_line_nr += 1;
                }
            }

            for entry in req.ref_list {
                match orig_lines.get(orig_line_nr) {
                    Some(entry_line) if entry_line.starts_with('-') => {
                        // Note: Replaced old entries with new ones.
                        new_lines.push(entry.to_string());
                        orig_line_nr += 1;
                    }
                    Some(_) | None => {
                        new_lines.push(entry.to_string());
                    }
                }
            }
        }

        while let Some(orig_line) = orig_lines.get(orig_line_nr) {
            new_lines.push(orig_line.to_string());
            orig_line_nr += 1;
        }

        // Add one more line, because `join()` consumes last line break.
        if orig_content.ends_with('\n') {
            new_lines.push("".to_string());
        }

        // Note: Always exit if we fail to write, because this is a severe problem.
        std::fs::write(filepath, new_lines.join("\n"))
            .map_err(|_| logid::pipe!(SyncError::AccessingWikiFile(filepath.clone())))?;
    }

    logid::log!(
        SyncInfo::Changed,
        "Wiki and project successfully synchronized."
    );

    Ok(())
}

/// Possible errors that may occure during synchronisation.
#[derive(Debug, thiserror::Error, logid::ErrLogId)]
pub enum SyncError {
    #[error("Failed to create the wiki from the given requirements folder.")]
    WikiSetup,
    #[error("Failed to count references from the given project folder.")]
    ReferenceCounting,
    #[error("Could not read and/or write to the requirements file '{}' in the wiki.", .0.to_string_lossy())]
    AccessingWikiFile(PathBuf),
}

impl From<WikiError> for SyncError {
    fn from(_value: WikiError) -> Self {
        SyncError::WikiSetup
    }
}

impl From<ReferencesError> for SyncError {
    fn from(_value: ReferencesError) -> Self {
        SyncError::ReferenceCounting
    }
}

/// Informations that may set during synchronisation.
#[derive(Debug, logid::InfoLogId)]
enum SyncInfo {
    /// Wiki and project already synchronized.
    Unchanged,
    /// Wiki and project successfully synchronized.
    Changed,
}
