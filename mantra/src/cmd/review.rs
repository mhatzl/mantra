use std::path::PathBuf;

use mantra_lang_tracing::ReqId;
use time::PrimitiveDateTime;

use crate::db::{DbError, MantraDb};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Review {
    pub name: String,
    #[serde(with = "super::review_date_format")]
    pub date: PrimitiveDateTime,
    pub reviewer: String,
    pub comment: Option<String>,
    pub requirements: Vec<VerifiedRequirement>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, clap::Args)]
pub struct ReviewConfig {
    pub reviews: Vec<PathBuf>,
}

pub async fn review(db: &MantraDb, cfg: ReviewConfig) -> Result<usize, ReviewError> {
    let mut review_cnt = 0;

    for review_file in &cfg.reviews {
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
        let review: Review = toml::from_str(&file_content).map_err(|err| {
            log::error!(
                "Failed parsing review file '{}': {}",
                review_file.display(),
                err
            );
            ReviewError::Parsing(review_file.to_path_buf())
        })?;

        let res = db.add_review(review).await.map_err(ReviewError::Db);

        if let Err(err) = res {
            log::error!("Adding review '{}' failed: {}", review_file.display(), err);
        }

        review_cnt += 1;
    }

    Ok(review_cnt)
}
