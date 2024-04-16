// setup db (migrate macro verwenden)

use std::path::PathBuf;

use mantra_lang_traits::ReqTrace;
use serde::{Deserialize, Serialize};
use sqlx::Pool;

pub use sqlx;

pub type DB = sqlx::sqlite::Sqlite;

#[derive(Debug)]
pub struct MantraDb {
    pool: Pool<DB>,
}

pub struct Requirement {
    pub id: String,
    pub origin: sqlx::types::Json<RequirementOrigin>,
}

#[derive(Deserialize, Serialize)]
pub enum RequirementOrigin {
    GitHub(GitHubReqOrigin),
    Jira(String),
}

#[derive(Deserialize, Serialize)]
pub struct GitHubReqOrigin {
    pub root: String,
    pub path: PathBuf,
    pub line: usize,
}

pub struct Config {
    /// URL to connect to a SQL database.
    /// Default is a SQLite file named `mantra.db` that is located under `.mantra/` at the workspace root.
    pub url: Option<String>,
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

impl MantraDb {
    pub async fn new(cfg: &Config) -> Result<Self, DbError> {
        let url = cfg.url.clone().unwrap_or("sqlite://mantra.db".to_string());
        let pool = Pool::<DB>::connect(&url)
            .await
            .map_err(|err| DbError::Connection(err.to_string()))?;

        MIGRATOR
            .run(&pool)
            .await
            .map_err(|err| DbError::Migration(err.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn add_reqs(&self, reqs: Vec<Requirement>) -> Result<(), DbError> {
        for req in &reqs {
            let res = sqlx::query!(
                "insert or replace into Requirements (id, origin) values ($1, $2)",
                req.id,
                req.origin
            )
            .execute(&self.pool)
            .await;

            if let Err(err) = res {
                return Err(DbError::Insertion(format!(
                    "Adding requirement '{}' failed with error: {}",
                    &req.id, err
                )));
            }
        }

        // hierarchy cannot be setup before due to foreign key constraint and possible "holes" in the hierarchy.
        // e.g. parent="req_id" child="req_id.test.first" with hole="req_id.test"
        for req in &reqs {
            if let Some((parent, _)) = req.id.rsplit_once('.') {
                let parent_exists =
                    sqlx::query!("select id from requirements where id = $1", parent)
                        .fetch_one(&self.pool)
                        .await
                        .is_ok();

                let existing_parent = if parent_exists {
                    parent.to_string()
                } else {
                    self.get_existing_parent(parent)
                        .await
                        .ok_or(DbError::Insertion(format!(
                            "Parent is missing for child='{}'.",
                            req.id
                        )))?
                };

                let res = sqlx::query!(
                    "insert or ignore into RequirementHierarchies (parent_id, child_id) values ($1, $2)",
                    existing_parent,
                    req.id,
                )
                .execute(&self.pool)
                .await;

                if let Err(err) = res {
                    return Err(DbError::Insertion(format!(
                        "Adding requirement hierarchy for parent='{}' and child='{}' failed with error: {}",
                        existing_parent, req.id, err
                    )));
                }
            }
        }

        Ok(())
    }

    async fn get_existing_parent(&self, mut id: &str) -> Option<String> {
        while let Some((parent, _)) = id.rsplit_once('.') {
            let parent_exists = sqlx::query!("select id from requirements where id = $1", parent)
                .fetch_one(&self.pool)
                .await
                .is_ok();

            if parent_exists {
                return Some(parent.to_string());
            } else {
                id = parent;
            }
        }

        None
    }

    pub async fn add_project(
        &self,
        project_name: &str,
        origin: ProjectOrigin,
    ) -> Result<(), DbError> {
        todo!()
    }

    pub async fn add_traces(
        &self,
        project_name: &str,
        filepath: &PathBuf,
        traces: Vec<ReqTrace>,
    ) -> Result<(), DbError> {
        todo!()
    }

    pub async fn add_coverage(
        &self,
        project_name: &str,
        filepath: &PathBuf,
        line: u32,
        req_id: &str,
    ) -> Result<(), DbError> {
        todo!()
    }

    pub async fn add_deprecated(&self, project_name: &str, req_id: &str) -> Result<(), DbError> {
        todo!()
    }

    pub async fn add_untraceable(&self, project_name: &str, req_id: &str) -> Result<(), DbError> {
        todo!()
    }

    pub async fn is_valid(&self) -> Result<(), DbError> {
        // validate db tables
        todo!()
    }

    pub fn pool(&self) -> &Pool<DB> {
        // akt workaround für custom queries
        &self.pool
    }
}

pub enum ProjectOrigin {
    GitRepo(GitRepoOrigin),
}

pub struct GitRepoOrigin {
    pub link: String,
    pub branch: Option<String>,
}

#[derive(Debug)]
pub enum DbError {
    Connection(String),
    Migration(String),
    Insertion(String),
}
