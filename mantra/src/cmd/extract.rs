use std::path::{Path, PathBuf};

use crate::db::{GitHubReqOrigin, MantraDb, Requirement};

use ignore::{types::TypesBuilder, WalkBuilder};
use regex::Regex;

pub async fn extract(db: &MantraDb, cfg: &Config) -> Result<(), ExtractError> {
    match cfg.origin {
        ExtractOrigin::GitHub => extract_github(db, &cfg.root, &cfg.link).await,
        ExtractOrigin::Jira => todo!(),
    }
}

async fn extract_github(db: &MantraDb, root: &Path, link: &str) -> Result<(), ExtractError> {
    if root.is_dir() {
        let walk = WalkBuilder::new(root)
            .types(
                TypesBuilder::new()
                    .add_defaults()
                    .select("markdown")
                    .build()
                    .expect("Could not create markdown file filter."),
            )
            .build();

        for dir_entry_res in walk {
            let dir_entry = match dir_entry_res {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            if dir_entry
                .file_type()
                .expect("No file type found for given entry. Note: stdin is not supported.")
                .is_file()
            {
                let filepath = dir_entry.path().to_string_lossy().to_string();
                let content = std::fs::read_to_string(dir_entry.path())
                    .map_err(|_| ExtractError::CouldNotAccessFile(filepath))?;

                let reqs = extract_from_wiki_content(&content, dir_entry.path(), link);

                if reqs.is_empty() {
                    // warn that no reqs were found
                    continue;
                }

                db.add_reqs(reqs).await.map_err(ExtractError::DbError)?
            }
        }
    } else {
        let filepath = root.to_string_lossy().to_string();
        let content = std::fs::read_to_string(root)
            .map_err(|_| ExtractError::CouldNotAccessFile(filepath))?;

        let reqs = extract_from_wiki_content(&content, root, link);

        if reqs.is_empty() {
            // warn that no reqs were found
        } else {
            db.add_reqs(reqs).await.map_err(ExtractError::DbError)?
        }
    }

    Ok(())
}

static REQ_ID_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

fn extract_from_wiki_content(content: &str, filepath: &Path, link: &str) -> Vec<Requirement> {
    let lines = content.lines();

    let mut reqs = Vec::new();
    let mut in_verbatim_context = false;

    let regex = REQ_ID_MATCHER.get_or_init(|| {
        Regex::new(r"^#+\s`(?<id>[^\s:]+)`:\s.+")
            .expect("Regex to match the requirement ID could **not** be created.")
    });

    for (line_nr, line) in lines.enumerate() {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            in_verbatim_context = !in_verbatim_context;
        }

        if !in_verbatim_context {
            if let Some(captures) = regex.captures(line) {
                let id = captures
                    .name("id")
                    .expect("`id` capture group was not in heading match.")
                    .as_str()
                    .to_string();

                reqs.push(Requirement {
                    id,
                    origin: crate::db::RequirementOrigin::GitHub(GitHubReqOrigin {
                        link: link.to_string(),
                        path: filepath.to_path_buf(),
                        line: line_nr + 1,
                    })
                    .into(),
                });
            }
        }
    }

    reqs
}

#[derive(Debug, Clone, clap::Args)]
#[group(id = "extract")]
pub struct Config {
    pub root: PathBuf,
    pub link: String,
    #[arg(value_enum)]
    pub origin: ExtractOrigin,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ExtractOrigin {
    GitHub,
    Jira,
}

#[derive(Debug)]
pub enum ExtractError {
    CouldNotAccessFile(String),
    DbError(crate::db::DbError),
}
