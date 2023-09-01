use std::path::{Path, PathBuf};

use regex::Regex;
use walkdir::WalkDir;

use crate::{
    global_param::GlobalParameter,
    wiki::{Wiki, WikiError},
};

/// Updates and adds *wiki-links* to all references found in the given project folder.
///
/// **Note:** The wiki itself may be given as project folder to update internal links to requirements.
///
/// [req:wiki.link.update]
pub fn link(params: &GlobalParameter) -> Result<(), LinkError> {
    let project_folder = &params.proj_folder;
    let mut wiki = Wiki::try_from(&params.req_folder)?;

    if let Some(url_prefix) = &params.wiki_url_prefix {
        wiki.set_url_prefix(url_prefix.clone());
    }

    if !project_folder.exists() {
        return logid::err!(LinkError::CouldNotFindProjectFolder(project_folder.clone(),));
    }

    if project_folder.is_dir() {
        let mut walk = WalkDir::new(project_folder)
            .into_iter()
            // TODO: add filter option using ignore files
            .filter_entry(|entry| {
                entry.file_name().to_string_lossy() != "target"
                    && entry.file_name().to_string_lossy() != ".git"
                    && entry.file_name().to_string_lossy() != "Cargo.lock"
                    && entry.file_name().to_string_lossy() != ".vscode"
            });
        while let Some(Ok(dir_entry)) = walk.next() {
            if dir_entry.file_type().is_file() {
                let content = std::fs::read_to_string(dir_entry.path()).map_err(|_| {
                    logid::pipe!(LinkError::CouldNotAccessFile(
                        dir_entry.path().to_path_buf()
                    ))
                })?;

                let new_content = update_links(&wiki, dir_entry.path(), &content)?;

                std::fs::write(dir_entry.path(), new_content).map_err(|_| {
                    logid::pipe!(LinkError::CouldNotAccessFile(project_folder.clone()))
                })?;
            }
        }
    } else {
        let content = std::fs::read_to_string(project_folder)
            .map_err(|_| logid::pipe!(LinkError::CouldNotAccessFile(project_folder.clone())))?;

        let new_content = update_links(&wiki, project_folder, &content)?;

        std::fs::write(project_folder, new_content)
            .map_err(|_| logid::pipe!(LinkError::CouldNotAccessFile(project_folder.clone())))?;
    }

    Ok(())
}

/// Holds the regex matcher for requirement references with optional wiki-links.
static WIKI_LINK_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

fn update_links(wiki: &Wiki, filepath: &Path, content: &str) -> Result<String, LinkError> {
    let link_regex = WIKI_LINK_MATCHER.get_or_init(|| {
        // [mantra:ignore_next]
        Regex::new(r"\[req:(?<req_id>[^\]\s]+)\](?:\((?<link>[^\)]+)\))?")
            .expect("Regex to match requirement references and optional wiki-links could **not** be created.")
    });

    let lines = content.lines();
    let mut ignore_match = false;
    let mut new_content: Vec<String> = Vec::new();

    for (line_nr, line) in lines.enumerate() {
        if line.contains("[mantra:ignore_next]") {
            ignore_match = true;
        }

        let mut line_updated = false;
        let mut last_grp_end = 0;
        let mut new_line = String::with_capacity(line.len());

        for captures in link_regex.captures_iter(line) {
            if ignore_match {
                ignore_match = false;
                continue;
            }

            let req_id = captures
                .name("req_id")
                .expect("`req_id` capture group was not in reference match.")
                .as_str()
                .to_string();

            let update_link = match captures.name("link") {
                Some(existing_link) => !wiki
                    .is_valid_link(&req_id, existing_link.as_str())
                    .map_err(|_| LinkError::WikiLink {
                        req_id: req_id.clone(),
                        filepath: filepath.to_path_buf(),
                        line_nr,
                    })?,
                None => true,
            };

            if update_link {
                line_updated = true;

                let link = wiki.wiki_link(&req_id).map_err(|_| LinkError::WikiLink {
                    req_id: req_id.clone(),
                    filepath: filepath.to_path_buf(),
                    line_nr,
                })?;
                let global_match = captures
                    .get(0)
                    .expect("Guaranteed to be `Some` according to doc.");

                let content_slice = &line[last_grp_end..global_match.start()];
                new_line.push_str(content_slice);
                new_line.push_str(&format!("[req:{}]({})", req_id, link));

                last_grp_end = global_match.end();
            }
        }

        if !line_updated {
            new_content.push(line.to_string());
        } else {
            if last_grp_end < line.len() {
                let content_slice = &line[last_grp_end..line.len()];
                new_line.push_str(content_slice);
            }

            new_content.push(new_line);
        }
    }

    if content.ends_with('\n') {
        new_content.push("".to_string());
    }

    Ok(new_content.join("\n"))
}

/// Possible errors that may occure during synchronisation.
#[derive(Debug, thiserror::Error, logid::ErrLogId)]
pub enum LinkError {
    #[error("Failed to create the wiki from the given requirements folder.")]
    WikiSetup,

    #[error("Failed to update the wiki-link for requirement ID '{}' in file '{}' at line '{}'.", .req_id, .filepath.to_string_lossy(), .line_nr + 1)]
    WikiLink {
        req_id: String,
        filepath: PathBuf,
        line_nr: usize,
    },

    #[error("Could not access file '{}' in the project folder.", .0.to_string_lossy())]
    CouldNotAccessFile(PathBuf),

    #[error("Could not find project folder '{}'.", .0.to_string_lossy())]
    CouldNotFindProjectFolder(PathBuf),

    // Note: +1 for line number, because internally, lines start at index 0.
    #[error("Requirement ID '{}' referenced in file '{}' at line '{}' not found in the wiki.", .req_id, .filepath.to_string_lossy(), .line_nr + 1)]
    ReqNotInWiki {
        req_id: String,
        filepath: PathBuf,
        line_nr: usize,
    },
}

impl From<WikiError> for LinkError {
    fn from(_value: WikiError) -> Self {
        LinkError::WikiSetup
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn setup_wiki() -> Wiki {
        let filename = "5-REQ-wiki.link.md";
        let content = r#"
# wiki.link: Some Title

**References:**

- in branch main: 2
        "#;

        let mut wiki = Wiki::try_from((PathBuf::from(filename), content)).unwrap();
        wiki.set_url_prefix("https://github.com/mhatzl/mantra/wiki".to_string());
        wiki
    }

    #[test]
    fn update_invalid_link_bad_anchor() {
        let wiki = setup_wiki();
        let filename = "references_wiki_link.rs";
        let content = "[req:wiki.link](https://github.com/mhatzl/mantra/wiki/5-REQ-wiki.link#documentation-for-requirements)";

        let new_content = update_links(&wiki, &PathBuf::from(filename), content).unwrap();

        assert_eq!(
            new_content,
            "[req:wiki.link](https://github.com/mhatzl/mantra/wiki/5-REQ-wiki.link#wikilink-some-title)",
            "Invalid link not updated correctly."
        );
    }
}
