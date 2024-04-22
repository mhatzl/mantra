// setup db (migrate macro verwenden)

use std::path::{Path, PathBuf};

use mantra_lang_tracing::TraceEntry;
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
    pub link: String,
    pub path: PathBuf,
    pub line: usize,
}

#[derive(Debug, Clone, clap::Args)]
#[group(id = "db")]
pub struct Config {
    /// URL to connect to a SQL database.
    /// Default is a SQLite file named `mantra.db` that is located at the workspace root.
    #[arg(long, alias = "db-url")]
    pub url: Option<String>,
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

impl MantraDb {
    pub async fn new(cfg: &Config) -> Result<Self, DbError> {
        let url = cfg
            .url
            .clone()
            .unwrap_or("sqlite://mantra.db?mode=rwc".to_string());
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
        let ser_origin: sqlx::types::Json<ProjectOrigin> = origin.into();

        let _ = sqlx::query!(
            "insert or replace into Projects (name, origin) values ($1, $2)",
            project_name,
            ser_origin
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            DbError::Insertion(format!(
                "Adding project '{}' failed with error: {}",
                project_name, err
            ))
        })?;

        Ok(())
    }

    pub async fn add_traces(
        &self,
        project_name: &str,
        filepath: &Path,
        traces: &[TraceEntry],
    ) -> Result<(), DbError> {
        let file = filepath.to_string_lossy();

        for trace in traces {
            let ids = trace.ids();
            let line = trace.line();

            for id in ids {
                let _ = sqlx::query!(
                    "insert or ignore into Traces (req_id, project_name, filepath, line) values ($1, $2, $3, $4)",
                    id,
                    project_name,
                    file,
                    line,
                )
                .execute(&self.pool)
                .await
                .map_err(|err| {
                    DbError::Insertion(format!(
                        "Adding trace for id='{}', project='{}', file='{}', line='{}' failed with error: {}",
                        id, project_name, file, line, err
                    ))
                })?;
            }
        }

        Ok(())
    }

    pub async fn add_coverage(
        &self,
        project_name: &str,
        test_name: &str,
        filepath: &Path,
        line: u32,
        req_id: &str,
    ) -> Result<(), DbError> {
        let file = filepath.to_string_lossy();
        let _ = sqlx::query!(
                "insert or ignore into Coverage (req_id, project_name, test_name, filepath, line) values ($1, $2, $3, $4, $5)",
                req_id,
                project_name,
                test_name,
                file,
                line,
            )
            .execute(&self.pool)
            .await
            .map_err(|err| {
                DbError::Insertion(format!(
                    "Adding coverage for id='{}', project='{}', test='{}', file='{}', line='{}' failed with error: {}",
                    req_id, project_name, test_name, file, line, err
                ))
            })?;

        Ok(())
    }

    pub async fn add_test(
        &self,
        name: &str,
        project_name: &str,
        filepath: &Path,
        line: u32,
    ) -> Result<(), DbError> {
        let file = filepath.to_string_lossy();
        let _ = sqlx::query!(
                "insert or ignore into Tests (name, project_name, filepath, line) values ($1, $2, $3, $4)",
                name,
                project_name,
                file,
                line,
            )
            .execute(&self.pool)
            .await
            .map_err(|err| {
                DbError::Insertion(format!(
                    "Adding test for test='{}', project='{}', file='{}', line='{}' failed with error: {}",
                    name, project_name, file, line, err
                ))
            })?;

        Ok(())
    }

    pub async fn add_deprecated(&self, req_id: &str, project_name: &str) -> Result<(), DbError> {
        let _ = sqlx::query!(
            "insert or replace into DeprecatedRequirements (req_id, project_name) values ($1, $2)",
            req_id,
            project_name
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            DbError::Insertion(format!(
                "Adding deprecated requirement='{}' for project='{}' failed with error: {}",
                req_id, project_name, err
            ))
        })?;

        Ok(())
    }

    pub async fn add_untraceable(&self, req_id: &str, project_name: &str) -> Result<(), DbError> {
        let _ = sqlx::query!(
            "insert or replace into UntraceableRequirements (req_id, project_name) values ($1, $2)",
            req_id,
            project_name
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            DbError::Insertion(format!(
                "Adding untraceable requirement='{}' for project='{}' failed with error: {}",
                req_id, project_name, err
            ))
        })?;

        Ok(())
    }

    pub async fn is_valid(&self) -> Result<(), DbError> {
        // validate db tables
        todo!()
    }

    pub fn pool(&self) -> &Pool<DB> {
        // workaround for custom queries
        &self.pool
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, clap::Parser)]
pub enum ProjectOrigin {
    GitRepo(GitRepoOrigin),
}

#[derive(Debug, Clone, Deserialize, Serialize, clap::Args)]
pub struct GitRepoOrigin {
    pub link: String,
    pub branch: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DbError {
    Connection(String),
    Migration(String),
    Insertion(String),
}
