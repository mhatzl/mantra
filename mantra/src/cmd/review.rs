use std::path::PathBuf;

use mantra_schema::{requirements::ReqId, reviews::ReviewSchema};
use time::PrimitiveDateTime;

use crate::db::{DbError, MantraDb};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Review {
    pub name: String,
    #[serde(with = "super::review_date_format")]
    #[schemars(
        with = "String",
        regex(
            pattern = r"(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2}) (?<hour>\d{2}):(?<minute>\d{2})(?<second>:\d{2}(?<subsecond>\.\d{3})?)?"
        )
    )]
    pub date: PrimitiveDateTime,
    pub reviewer: String,
    pub comment: Option<String>,
    pub requirements: Vec<VerifiedRequirement>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct VerifiedRequirement {
    pub id: ReqId,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ReviewError {
    #[error("Could not read file '{}'.", .0.display())]
    ReadingFile(PathBuf),
    #[error("File '{}' is not a valid review.", .0.display())]
    Parsing(PathBuf),
    #[error("{}", .0)]
    Db(DbError),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewConfig {
    #[serde(
        alias = "filepaths",
        alias = "external-files",
        alias = "external-filepaths"
    )]
    pub files: Vec<PathBuf>,
}

pub async fn collect(db: &MantraDb, cfg: ReviewConfig) -> Result<usize, ReviewError> {
    let mut review_cnt = 0;

    for review_file in &cfg.files {
        if !matches!(
            review_file.extension().and_then(|s| s.to_str()),
            Some("toml")
        ) {
            log::warn!(
                "Only TOML format is supported for reviews. Skipped file '{}'.",
                review_file.display()
            );
            continue;
        }

        let file_content = std::fs::read_to_string(review_file)
            .map_err(|_| ReviewError::ReadingFile(review_file.to_path_buf()))?;
        let review: ReviewSchema = toml::from_str(&file_content).map_err(|err| {
            log::error!(
                "Failed parsing review file '{}': {}",
                review_file.display(),
                err
            );
            ReviewError::Parsing(review_file.to_path_buf())
        })?;

        if db.review_exists(&review.name, &review.date).await {
            log::info!(
                "Review '{}' already in the database.",
                review_file.display()
            );
        } else {
            let res = db.add_review(review).await.map_err(ReviewError::Db);

            if let Err(err) = res {
                log::error!("Adding review '{}' failed: {}", review_file.display(), err);
            }

            review_cnt += 1;
        }
    }

    Ok(review_cnt)
}

pub async fn collect_from_schema(db: &MantraDb, review: ReviewSchema) -> Result<(), ReviewError> {
    db.add_review(review).await.map_err(ReviewError::Db)
}
