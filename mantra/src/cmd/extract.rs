use std::path::{Path, PathBuf};

use crate::db::{GitHubReqOrigin, MantraDb, Requirement, RequirementChanges};

use ignore::{types::TypesBuilder, WalkBuilder};
use regex::Regex;

#[derive(Debug, Clone, clap::Args, serde::Deserialize)]
#[group(id = "extract")]
pub struct Config {
    #[arg(alias = "local-path")]
    pub root: PathBuf,
    pub link: String,
    #[arg(value_enum)]
    pub origin: ExtractOrigin,
    #[arg(long, alias = "version")]
    pub major_version: Option<usize>,
}

#[derive(Debug, Clone, clap::ValueEnum, serde::Deserialize)]
pub enum ExtractOrigin {
    GitHub,
    Jira,
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("Could not access file '{}'.", .0)]
    CouldNotAccessFile(String),
    #[error("{}", .0)]
    DbError(crate::db::DbError),
}

pub async fn extract(db: &MantraDb, cfg: &Config) -> Result<RequirementChanges, ExtractError> {
    match cfg.origin {
        ExtractOrigin::GitHub => extract_github(db, &cfg.root, &cfg.link, cfg.major_version).await,
        ExtractOrigin::Jira => todo!(),
    }
}

async fn extract_github(
    db: &MantraDb,
    root: &Path,
    link: &str,
    version: Option<usize>,
) -> Result<RequirementChanges, ExtractError> {
    let mut reqs = Vec::new();

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

                reqs.append(&mut extract_from_wiki_content(
                    &content,
                    dir_entry.path(),
                    link,
                    version,
                ));
            }
        }
    } else {
        let filepath = root.to_string_lossy().to_string();
        let content = std::fs::read_to_string(root)
            .map_err(|_| ExtractError::CouldNotAccessFile(filepath))?;

        reqs = extract_from_wiki_content(&content, root, link, version);
    }

    if reqs.is_empty() {
        log::warn!("No requirements were found.");

        let changes = RequirementChanges {
            new_generation: db.max_req_generation().await,
            ..Default::default()
        };
        Ok(changes)
    } else {
        db.add_reqs(reqs).await.map_err(ExtractError::DbError)
    }
}

static REQ_ID_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

fn extract_from_wiki_content(
    content: &str,
    filepath: &Path,
    link: &str,
    version: Option<usize>,
) -> Vec<Requirement> {
    let lines = content.lines();

    let mut reqs = Vec::new();
    let mut in_verbatim_context = false;

    let regex = REQ_ID_MATCHER.get_or_init(|| {
        Regex::new(
            r"^#{1,6}\s`(?<id>[^\s:]+)`(?:\((?:v(?<version>\d{1,7}):)?(?<annotation>[^\)]+)\))?:\s.+",
        )
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

                let mut annotation = captures.name("annotation").map(|c| c.as_str().to_string());
                let extracted_version: Option<usize> = captures.name("version").map(|c| {
                    c.as_str()
                        .parse()
                        .expect("Matched digits must fit into *usize*.")
                });

                if let Some(version) = version {
                    if let Some(extracted_version) = extracted_version {
                        if version < extracted_version {
                            annotation = None;
                        }
                    }
                }

                reqs.push(Requirement {
                    id,
                    origin: crate::db::RequirementOrigin::GitHub(GitHubReqOrigin {
                        link: link.to_string(),
                        path: filepath.to_path_buf(),
                        line: line_nr + 1,
                    })
                    .into(),
                    annotation,
                });
            }
        }
    }

    reqs
}
