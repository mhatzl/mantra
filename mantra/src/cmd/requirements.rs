use std::path::{Path, PathBuf};

use crate::db::{MantraDb, RequirementChanges};

use ignore::{types::TypesBuilder, WalkBuilder};
use mantra_schema::requirements::{Requirement, RequirementSchema};
use regex::Regex;

#[derive(Debug, Clone, clap::Subcommand, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    FromWiki(WikiConfig),
    FromSchema { filepath: PathBuf },
}

#[derive(Debug, Clone, clap::Args, serde::Serialize, serde::Deserialize)]
pub struct WikiConfig {
    #[arg(alias = "local-path")]
    pub root: PathBuf,
    pub link: String,
    #[arg(long, alias = "version")]
    #[serde(alias = "version", alias = "major-version")]
    pub major_version: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
pub enum RequirementsError {
    #[error("Could not access file '{}'.", .0)]
    CouldNotAccessFile(String),
    #[error("{}", .0)]
    Deserialize(serde_json::Error),
    #[error("{}", .0)]
    DbError(crate::db::DbError),
}

pub async fn collect(db: &MantraDb, fmt: &Format) -> Result<RequirementChanges, RequirementsError> {
    match fmt {
        Format::FromWiki(wiki_cfg) => {
            collect_from_wiki(db, &wiki_cfg.root, &wiki_cfg.link, wiki_cfg.major_version).await
        }
        Format::FromSchema { filepath } => {
            let content = tokio::fs::read_to_string(filepath).await.map_err(|_| {
                RequirementsError::CouldNotAccessFile(filepath.display().to_string())
            })?;
            let schema = serde_json::from_str(&content).map_err(RequirementsError::Deserialize)?;
            collect_from_schema(db, schema).await
        }
    }
}

pub async fn collect_from_schema(
    db: &MantraDb,
    schema: RequirementSchema,
) -> Result<RequirementChanges, RequirementsError> {
    db.add_reqs(schema.requirements)
        .await
        .map_err(RequirementsError::DbError)
}

async fn collect_from_wiki(
    db: &MantraDb,
    root: &Path,
    link: &str,
    version: Option<usize>,
) -> Result<RequirementChanges, RequirementsError> {
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
                let content = std::fs::read_to_string(dir_entry.path()).map_err(|_| {
                    RequirementsError::CouldNotAccessFile(dir_entry.path().display().to_string())
                })?;

                let file_stem = dir_entry
                    .path()
                    .file_stem()
                    .expect("Filepath is valid filename.")
                    .to_string_lossy()
                    .replace(char::is_whitespace, "-");
                let link = format!("{}/{}", link, file_stem);

                reqs.append(&mut requirements_from_wiki_content(
                    &content, &link, version,
                ));
            }
        }
    } else {
        let content = std::fs::read_to_string(root)
            .map_err(|_| RequirementsError::CouldNotAccessFile(root.display().to_string()))?;

        reqs = requirements_from_wiki_content(&content, link, version);
    }

    if reqs.is_empty() {
        log::warn!("No requirements were found.");

        let changes = RequirementChanges {
            new_generation: db.max_req_generation().await,
            ..Default::default()
        };
        Ok(changes)
    } else {
        db.add_reqs(reqs).await.map_err(RequirementsError::DbError)
    }
}

static REQ_ID_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

fn requirements_from_wiki_content(
    content: &str,
    link: &str,
    version: Option<usize>,
) -> Vec<Requirement> {
    let lines = content.lines();

    let mut reqs = Vec::new();
    let mut in_verbatim_context = false;

    let regex = REQ_ID_MATCHER.get_or_init(|| {
        Regex::new(
            r"^#{1,6}\s`(?<id>[^\s:]+)`(?:\((?:v(?<version>\d{1,7}):)?(?<marker>[^\)]+)\))?:\s+(?<title>[^\n]+)",
        )
        .expect("Regex to match the requirement ID could **not** be created.")
    });

    for line in lines {
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

                let mut marker = captures.name("marker").map(|c| c.as_str().to_string());
                let extracted_version: Option<usize> = captures.name("version").map(|c| {
                    c.as_str()
                        .parse()
                        .expect("Matched digits must fit into *usize*.")
                });

                if let Some(version) = version {
                    if let Some(extracted_version) = extracted_version {
                        if version < extracted_version {
                            marker = None;
                        }
                    }
                }

                let manual = marker == Some("manual".to_string());
                let deprecated = marker == Some("deprecated".to_string());

                let title = captures
                    .name("title")
                    .expect("`title` capture group was not in heading match.")
                    .as_str()
                    .to_string();

                reqs.push(Requirement {
                    id,
                    title,
                    link: link.to_string(),
                    info: None,
                    manual,
                    deprecated,
                    parents: None,
                });
            }
        }
    }

    reqs
}
