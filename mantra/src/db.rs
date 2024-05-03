// setup db (migrate macro verwenden)

use std::path::{Path, PathBuf};

use mantra_lang_tracing::TraceEntry;
use serde::{Deserialize, Serialize};
use sqlx::Pool;

pub use sqlx;

use crate::{
    cfg::{
        DeleteCoverageConfig, DeleteDeprecatedConfig, DeleteManualRequirementsConfig,
        DeleteReqsConfig, DeleteTracesConfig,
    },
    cmd::coverage::TestRunConfig,
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
    #[error("Failed to update table content. Cause: {}", .0)]
    Update(String),
    #[error("The database contains invalid data. Cause: {}", .0)]
    Validate(String),
    #[error("Foreign key violation: {}", .0)]
    ForeignKeyViolation(String),
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

    pub async fn add_traces(
        &self,
        filepath: &Path,
        root: Option<&Path>,
        traces: &[TraceEntry],
    ) -> Result<(), DbError> {
        let file = if let Some(root_path) = root {
            get_relative_path(root_path, filepath)?
                .display()
                .to_string()
        } else {
            filepath.display().to_string()
        };

        for trace in traces {
            let ids = trace.ids();
            let line = trace.line();

            for id in ids {
                let _ = sqlx::query!(
                    "insert or ignore into Traces (req_id, filepath, line) values ($1, $2, $3)",
                    id,
                    file,
                    line,
                )
                .execute(&self.pool)
                .await
                .map_err(|err| {
                    DbError::Insertion(format!(
                        "Adding trace for id='{}', file='{}', line='{}' failed with error: {}",
                        id, file, line, err
                    ))
                })?;
            }
        }

        Ok(())
    }

    pub async fn add_coverage(
        &self,
        test_run: &TestRunConfig,
        test_name: &str,
        filepath: &Path,
        line: u32,
        req_id: &str,
    ) -> Result<(), DbError> {
        // Note: filepath is already *fix* due to how the "file!()" macro works
        let file = filepath.display().to_string();
        let query_result = sqlx::query!(
                "insert or ignore into TestCoverage (req_id, test_run_name, test_run_date, test_name, filepath, line) values ($1, $2, $3, $4, $5, $6)",
                req_id,
                test_run.name,
                test_run.date,
                test_name,
                file,
                line,
            )
            .execute(&self.pool)
            .await;

        if let Err(sqlx::Error::Database(sqlx_db_error)) = &query_result {
            if sqlx_db_error.kind() == sqlx::error::ErrorKind::ForeignKeyViolation {
                return Err(DbError::ForeignKeyViolation(
                    sqlx_db_error.message().to_string(),
                ));
            }
        }

        query_result.map_err(|err| {
                DbError::Insertion(format!(
                    "Adding coverage for id='{}', test-run='{}' at {}, test='{}', file='{}', line='{}' failed with error: {}",
                    req_id, test_run.name, test_run.date, test_name, file, line, err
                ))
            })?;

        Ok(())
    }

    pub async fn add_test(
        &self,
        test_run: &TestRunConfig,
        name: &str,
        filepath: &Path,
        line: u32,
        passed: Option<bool>,
    ) -> Result<(), DbError> {
        let file = filepath.display().to_string();
        let _ = sqlx::query!(
                "insert or ignore into Tests (name, test_run_name, test_run_date, filepath, line, passed) values ($1, $2, $3, $4, $5, $6)",
                name,
                test_run.name,
                test_run.date,
                file,
                line,
                passed,
            )
            .execute(&self.pool)
            .await
            .map_err(|err| {
                DbError::Insertion(format!(
                    "Adding test for test='{}', test-run='{}' at {}, file='{}', line='{}' failed with error: {}",
                    name, test_run.name, test_run.date, file, line, err
                ))
            })?;

        Ok(())
    }

    pub async fn test_passed(&self, test_run: &TestRunConfig, name: &str) -> Result<(), DbError> {
        let _ = sqlx::query!(
            "update Tests set passed = $1 where name = $2 and test_run_name = $3 and test_run_date = $4",
            Some(true),
            name,
            test_run.name,
            test_run.date,
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            DbError::Update(format!(
                "Could not set 'passed = true' for test='{}' in test-run='{}' at {}. Cause: {}",
                name, test_run.name, test_run.date, err
            ))
        })?;

        Ok(())
    }

    pub async fn add_test_run(
        &self,
        test_run: &TestRunConfig,
        ser_logs: &str,
    ) -> Result<(), DbError> {
        let _ = sqlx::query!(
            "insert or ignore into TestRuns (name, date, logs) values ($1, $2, $3)",
            test_run.name,
            test_run.date,
            ser_logs
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            DbError::Insertion(format!(
                "Adding test-run with name='{}' and date='{}' failed with error: {}",
                test_run.name, test_run.date, err
            ))
        })?;

        Ok(())
    }

    pub async fn update_nr_of_tests(
        &self,
        test_run: &TestRunConfig,
        nr_of_tests: u32,
    ) -> Result<(), DbError> {
        let _ = sqlx::query!(
            "update TestRuns set nr_of_tests = $1 where name = $2 and date = $3 and nr_of_tests is null",
            nr_of_tests,
            test_run.name,
            test_run.date,
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            DbError::Update(format!(
                "Could not set 'nr_of_tests' for test-run='{}' at {}. Cause: {}",
                test_run.name, test_run.date, err
            ))
        })?;

        Ok(())
    }

    pub async fn add_deprecated(&self, req_id: &str) -> Result<(), DbError> {
        let _ = sqlx::query!(
            "insert or replace into DeprecatedRequirements (req_id) values ($1)",
            req_id
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            DbError::Insertion(format!(
                "Adding deprecated requirement='{}' failed with error: {}",
                req_id, err
            ))
        })?;

        Ok(())
    }

    pub async fn add_manual_req(&self, req_id: &str) -> Result<(), DbError> {
        let _ = sqlx::query!(
            "insert or replace into ManualRequirements (req_id) values ($1)",
            req_id
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            DbError::Insertion(format!(
                "Adding manual requirement='{}' failed with error: {}",
                req_id, err
            ))
        })?;

        Ok(())
    }

    pub async fn is_valid(&self) -> Result<(), DbError> {
        let traced_deprecated = sqlx::query!("select t.req_id from Traces as t, DeprecatedRequirements as dr where t.req_id = dr.req_id limit 5").fetch_all(&self.pool).await.map_err(|err| DbError::Validate(err.to_string()))?;

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
                    let _ = sqlx::query!("delete from TestCoverage where req_id = $1", id)
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
                    let _ = sqlx::query!("delete from ManuallyVerified where req_id = $1", id)
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;
                    let _ = sqlx::query!("delete from ManualRequirements where req_id = $1", id)
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;
                    let _ = sqlx::query!("delete from Requirements where id = $1", id)
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;
                }

                let _ = sqlx::query!(
                    "delete from Tests where name not in (select test_name from TestCoverage)"
                )
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;

                let _ =
                sqlx::query!("delete from TestRuns where (name, date) not in (select test_run_name, test_run_date from TestCoverage)")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ =
                sqlx::query!("delete from Reviews where (name, date) not in (select review_name, review_date from ManuallyVerified)")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
            None => {
                let _ = sqlx::query!("delete from RequirementHierarchies")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from TestCoverage")
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
                let _ = sqlx::query!("delete from TestRuns")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from DeprecatedRequirements")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from ManuallyVerified")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from ManualRequirements")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Reviews")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from Requirements")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        }

        Ok(())
    }

    pub async fn delete_traces(&self, cfg: &DeleteTracesConfig) -> Result<(), DbError> {
        let ids = cfg.req_ids.as_deref().unwrap_or_default();

        if ids.is_empty() {
            let _ = sqlx::query!("delete from TestCoverage")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            let _ = sqlx::query!("delete from Tests")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            let _ = sqlx::query!("delete from TestRuns")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            let _ = sqlx::query!("delete from Traces")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
        } else {
            for id in ids {
                let _ = sqlx::query!("delete from TestCoverage where req_id = $1", id)
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
                "delete from Tests where name not in (select test_name from TestCoverage)"
            )
            .execute(&self.pool)
            .await
            .map_err(|err| DbError::Delete(err.to_string()))?;

            let _ = sqlx::query!(
                "delete from TestRuns where (name, date) not in (select test_run_name, test_run_date from TestCoverage)"
            )
            .execute(&self.pool)
            .await
            .map_err(|err| DbError::Delete(err.to_string()))?;
        }

        Ok(())
    }

    pub async fn delete_deprecated(&self, cfg: &DeleteDeprecatedConfig) -> Result<(), DbError> {
        let ids = cfg.req_ids.as_deref().unwrap_or_default();

        if ids.is_empty() {
            let _ = sqlx::query!("delete from DeprecatedRequirements")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
        } else {
            for id in ids {
                let _ = sqlx::query!("delete from DeprecatedRequirements where req_id = $1", id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        }

        Ok(())
    }

    pub async fn delete_manual_reqs(
        &self,
        cfg: &DeleteManualRequirementsConfig,
    ) -> Result<(), DbError> {
        let ids = cfg.req_ids.as_deref().unwrap_or_default();

        if ids.is_empty() {
            let _ = sqlx::query!("delete from ManuallyVerified")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            let _ = sqlx::query!("delete from ManualRequirements")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            let _ = sqlx::query!("delete from Reviews")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
        } else {
            for id in ids {
                let _ = sqlx::query!("delete from ManuallyVerified where req_id = $1", id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
                let _ = sqlx::query!("delete from ManualRequirements where req_id = $1", id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }

            let _ =
                sqlx::query!("delete from Reviews where (name, date) not in (select review_name, review_date from ManuallyVerified)")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
        }

        Ok(())
    }

    pub async fn delete_coverage(&self, cfg: &DeleteCoverageConfig) -> Result<(), DbError> {
        let ids = cfg.req_ids.as_deref().unwrap_or_default();

        if ids.is_empty() {
            let _ = sqlx::query!("delete from TestCoverage")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            let _ = sqlx::query!("delete from Tests")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            let _ = sqlx::query!("delete from TestRuns")
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
        } else {
            for id in ids {
                let _ = sqlx::query!("delete from TestCoverage where req_id = $1", id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }

            let _ = sqlx::query!(
                "delete from Tests where name not in (select test_name from TestCoverage)"
            )
            .execute(&self.pool)
            .await
            .map_err(|err| DbError::Delete(err.to_string()))?;

            let _ = sqlx::query!(
                "delete from TestRuns where (name, date) not in (select test_run_name, test_run_date from TestCoverage)"
            )
            .execute(&self.pool)
            .await
            .map_err(|err| DbError::Delete(err.to_string()))?;
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
