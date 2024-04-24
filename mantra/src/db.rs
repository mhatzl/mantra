// setup db (migrate macro verwenden)

use std::path::{Path, PathBuf};

use mantra_lang_tracing::TraceEntry;
use serde::{Deserialize, Serialize};
use sqlx::Pool;

pub use sqlx;

use crate::cfg::{
    DeleteCoverageConfig, DeleteDeprecatedConfig, DeleteProjectConfig, DeleteReqsConfig,
    DeleteTracesConfig, DeleteUntraceableConfig,
};

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

#[derive(Debug, Clone, Deserialize, Serialize, clap::Parser)]
pub enum ProjectOrigin {
    GitRepo(GitRepoOrigin),
}

#[derive(Debug, Clone, Deserialize, Serialize, clap::Args)]
pub struct GitRepoOrigin {
    pub link: String,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, clap::Args)]
#[group(id = "db")]
pub struct Config {
    /// URL to connect to a SQL database.
    /// Default is a SQLite file named `mantra.db` that is located at the workspace root.
    #[arg(long, alias = "db-url")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DbError {
    #[error("Could not get connection to database. Cause: {}", .0)]
    Connection(String),
    #[error("Could not run migration on database. Cause: {}", .0)]
    Migration(String),
    #[error("Could not insert data into database. Cause: {}", .0)]
    Insertion(String),
    #[error("Failed to make filepath relative to root. Cause: {}", .0)]
    RelativeFilepath(String),
    #[error("Failed to delete table content. Cause: {}", .0)]
    Delete(String),
    #[error("The database contains invalid data. Cause: {}", .0)]
    Validate(String),
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
                    self.get_req_parent(parent)
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

    async fn get_req_parent(&self, mut id: &str) -> Option<String> {
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
        root: &Path,
        filepath: &Path,
        traces: &[TraceEntry],
    ) -> Result<(), DbError> {
        let file = get_relative_path(root, filepath)?.display().to_string();

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
        // Note: filepath is already relative due to how the "file!()" macro works
        let file = filepath.display().to_string();
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
        let file = filepath.display().to_string();
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
        let traced_deprecated = sqlx::query!("select t.req_id from Traces as t, DeprecatedRequirements as dr where t.req_id = dr.req_id and t.project_name = dr.project_name limit 5").fetch_all(&self.pool).await.map_err(|err| DbError::Validate(err.to_string()))?;

        if traced_deprecated.is_empty() {
            Ok(())
        } else {
            Err(DbError::Validate(format!(
                "One or more deprecated requirements have trace entries. Requirement ids: `{}`.",
                traced_deprecated
                    .iter()
                    .map(|entry| entry.req_id.clone())
                    .collect::<Vec<_>>()
                    .join("`, `")
            )))
        }
    }

    pub fn pool(&self) -> &Pool<DB> {
        // workaround for custom queries
        &self.pool
    }

    pub async fn delete_reqs(&self, cfg: &DeleteReqsConfig) -> Result<(), DbError> {
        match &cfg.ids {
            Some(ids) => {
                for id in ids {
                    let _ = sqlx::query!(
                        "delete from RequirementHierarchies where parent_id = $1 or child_id = $1",
                        id
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                    let _ = sqlx::query!("delete from Coverage where req_id = $1", id)
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;

                    let _ = sqlx::query!("delete from Traces where req_id = $1", id)
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;

                    let _ =
                        sqlx::query!("delete from DeprecatedRequirements where req_id = $1", id)
                            .execute(&self.pool)
                            .await
                            .map_err(|err| DbError::Delete(err.to_string()))?;
                    let _ =
                        sqlx::query!("delete from UntraceableRequirements where req_id = $1", id)
                            .execute(&self.pool)
                            .await
                            .map_err(|err| DbError::Delete(err.to_string()))?;
                    let _ = sqlx::query!("delete from Requirements where id = $1", id)
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;
                }
            }
            None => {
                let _ = sqlx::query!("delete from RequirementHierarchies")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Coverage")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;

                let _ = sqlx::query!("delete from Traces")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;

                let _ = sqlx::query!("delete from DeprecatedRequirements")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from UntraceableRequirements")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Requirements")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        }

        let _ =
            sqlx::query!("delete from Tests where name not in (select test_name from Coverage)")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;

        Ok(())
    }

    pub async fn delete_projects(&self, cfg: &DeleteProjectConfig) -> Result<(), DbError> {
        match &cfg.projects {
            Some(projects) => {
                for project in projects {
                    let _ = sqlx::query!("delete from Coverage where project_name = $1", project)
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;

                    let _ = sqlx::query!("delete from Traces where project_name = $1", project)
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;
                    let _ = sqlx::query!("delete from Tests where project_name = $1", project)
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;
                    let _ = sqlx::query!(
                        "delete from DeprecatedRequirements where project_name = $1",
                        project
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                    let _ = sqlx::query!(
                        "delete from UntraceableRequirements where project_name = $1",
                        project
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                    let _ = sqlx::query!("delete from Projects where name = $1", project)
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;
                }
            }
            None => {
                let _ = sqlx::query!("delete from Coverage")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Traces")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Tests")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from DeprecatedRequirements")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from UntraceableRequirements")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Projects")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        }

        Ok(())
    }

    pub async fn delete_traces(&self, cfg: &DeleteTracesConfig) -> Result<(), DbError> {
        let ids = cfg.req_ids.as_deref().unwrap_or_default();
        let projects = cfg.projects.as_deref().unwrap_or_default();

        if ids.is_empty() && projects.is_empty() {
            let _ = sqlx::query!("delete from Coverage")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            let _ = sqlx::query!("delete from Tests")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            let _ = sqlx::query!("delete from Traces")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
        } else if ids.is_empty() {
            for project in projects {
                let _ = sqlx::query!("delete from Coverage where project_name = $1", project)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Tests where project_name = $1", project)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Traces where project_name = $1", project)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        } else if projects.is_empty() {
            for id in ids {
                let _ = sqlx::query!("delete from Coverage where req_id = $1", id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Traces where req_id = $1", id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }

            // tests have no associated requirement id, so deleting on "req_id" is not possible.
            // But if no coverage links to a test, it is safe to delete it
            let _ = sqlx::query!(
                "delete from Tests where name not in (select test_name from Coverage)"
            )
            .execute(&self.pool)
            .await
            .map_err(|err| DbError::Delete(err.to_string()))?;
        } else {
            for id in ids {
                for project in projects {
                    let _ = sqlx::query!(
                        "delete from Coverage where req_id = $1 and project_name = $2",
                        id,
                        project
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                    let _ = sqlx::query!(
                        "delete from Traces where req_id = $1 and project_name = $2",
                        id,
                        project
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                }
            }

            // tests have no associated requirement id, so deleting on "req_id" is not possible.
            // But if no coverage links to a test, it is safe to delete it
            let _ = sqlx::query!(
                "delete from Tests where name not in (select test_name from Coverage)"
            )
            .execute(&self.pool)
            .await
            .map_err(|err| DbError::Delete(err.to_string()))?;
        }

        Ok(())
    }

    pub async fn delete_deprecated(&self, cfg: &DeleteDeprecatedConfig) -> Result<(), DbError> {
        let ids = cfg.req_ids.as_deref().unwrap_or_default();
        let projects = cfg.projects.as_deref().unwrap_or_default();

        if ids.is_empty() && projects.is_empty() {
            let _ = sqlx::query!("delete from DeprecatedRequirements")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
        } else if ids.is_empty() {
            for project in projects {
                let _ = sqlx::query!(
                    "delete from DeprecatedRequirements where project_name = $1",
                    project
                )
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        } else if projects.is_empty() {
            for id in ids {
                let _ = sqlx::query!("delete from DeprecatedRequirements where req_id = $1", id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        } else {
            for id in ids {
                for project in projects {
                    let _ = sqlx::query!(
                        "delete from DeprecatedRequirements where req_id = $1 and project_name = $2",
                        id,
                        project
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                }
            }
        }

        Ok(())
    }

    pub async fn delete_untraceables(&self, cfg: &DeleteUntraceableConfig) -> Result<(), DbError> {
        let ids = cfg.req_ids.as_deref().unwrap_or_default();
        let projects = cfg.projects.as_deref().unwrap_or_default();

        if ids.is_empty() && projects.is_empty() {
            let _ = sqlx::query!("delete from UntraceableRequirements")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
        } else if ids.is_empty() {
            for project in projects {
                let _ = sqlx::query!(
                    "delete from UntraceableRequirements where project_name = $1",
                    project
                )
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        } else if projects.is_empty() {
            for id in ids {
                let _ = sqlx::query!("delete from UntraceableRequirements where req_id = $1", id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        } else {
            for id in ids {
                for project in projects {
                    let _ = sqlx::query!(
                        "delete from UntraceableRequirements where req_id = $1 and project_name = $2",
                        id,
                        project
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                }
            }
        }

        Ok(())
    }

    pub async fn delete_coverage(&self, cfg: &DeleteCoverageConfig) -> Result<(), DbError> {
        let tests = cfg.tests.as_deref().unwrap_or_default();
        let projects = cfg.projects.as_deref().unwrap_or_default();

        if tests.is_empty() && projects.is_empty() {
            let _ = sqlx::query!("delete from Coverage")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            let _ = sqlx::query!("delete from Tests")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
        } else if tests.is_empty() {
            for project in projects {
                let _ = sqlx::query!("delete from Coverage where project_name = $1", project)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Tests where project_name = $1", project)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        } else if projects.is_empty() {
            for test in tests {
                let _ = sqlx::query!("delete from Coverage where test_name = $1", test)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Tests where name = $1", test)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        } else {
            for test in tests {
                for project in projects {
                    let _ = sqlx::query!(
                        "delete from Coverage where test_name = $1 and project_name = $2",
                        test,
                        project
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                    let _ = sqlx::query!(
                        "delete from Tests where name = $1 and project_name = $2",
                        test,
                        project
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                }
            }
        }

        Ok(())
    }
}

pub fn get_relative_path(root: &Path, filepath: &Path) -> Result<PathBuf, DbError> {
    if root == filepath {
        match filepath.file_name() {
            Some(filename) => {
                return Ok(PathBuf::from(filename));
            }
            None => {
                return Err(DbError::RelativeFilepath(format!(
                    "Invalid filepath '{}' given relative to root path '{}'.",
                    filepath.display(),
                    root.display()
                )))
            }
        }
    }

    match filepath.strip_prefix(root) {
        Ok(relative_path) => Ok(relative_path.to_path_buf()),
        Err(_) => Err(DbError::RelativeFilepath(format!(
            "Root path '{}' is not the root of the given filepath '{}'.",
            root.display(),
            filepath.display()
        ))),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn valid_relative_filepath() {
        let root = PathBuf::from("src/");
        let filepath = PathBuf::from("src/cmd/mod.rs");

        let relative_path = get_relative_path(&root, &filepath).unwrap();

        assert_eq!(
            relative_path,
            PathBuf::from("cmd/mod.rs"),
            "Relative filepath not extracted correctly."
        )
    }

    #[test]
    fn filepath_is_root() {
        let root = PathBuf::from("src/main.rs");
        let filepath = PathBuf::from("src/main.rs");

        let relative_path = get_relative_path(&root, &filepath).unwrap();

        assert_eq!(
            relative_path,
            PathBuf::from("main.rs"),
            "Filename not used for root file."
        )
    }
}
