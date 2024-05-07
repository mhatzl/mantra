// setup db (migrate macro verwenden)

use std::path::{Path, PathBuf};

use mantra_lang_tracing::TraceEntry;
use serde::{Deserialize, Serialize};
use sqlx::Pool;

pub use sqlx;

use crate::{
    cfg::{DeleteCoverageConfig, DeleteReqsConfig, DeleteTracesConfig},
    cmd::coverage::TestRunConfig,
};

pub type DB = sqlx::sqlite::Sqlite;

#[derive(Debug)]
pub struct MantraDb {
    pool: Pool<DB>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    req_id: String,
    filepath: PathBuf,
    line: u32,
}

impl std::fmt::Display for Trace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "id=`{}`, file='{}', line='{}'",
            self.req_id,
            self.filepath.display(),
            self.line
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeletedTraces(Vec<Trace>);

impl std::ops::Deref for DeletedTraces {
    type Target = Vec<Trace>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for DeletedTraces {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::fmt::Display for DeletedTraces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            writeln!(f, "No trace was deleted.")
        } else {
            writeln!(f, "'{}' traces deleted:", self.len())?;
            for trace in &self.0 {
                writeln!(f, "- {trace}")?;
            }

            Ok(())
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TraceChanges {
    pub inserted: Vec<Trace>,
    pub unchanged_cnt: usize,
    pub new_generation: i64,
}

impl TraceChanges {
    pub fn merge(&mut self, other: &mut Self) {
        self.inserted.append(&mut other.inserted);
        self.unchanged_cnt += other.unchanged_cnt;
    }
}

impl std::fmt::Display for TraceChanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.inserted.is_empty() {
            if self.unchanged_cnt == 0 {
                writeln!(f, "No traces found.")?;
            } else {
                writeln!(f, "'{}' traces kept.", self.unchanged_cnt)?;
            }
        } else {
            writeln!(f, "'{}' traces added:", self.inserted.len())?;
            for trace in &self.inserted {
                writeln!(f, "- `{}`", trace)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub id: String,
    pub origin: sqlx::types::Json<RequirementOrigin>,
    pub annotation: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeletedRequirements(Vec<Requirement>);

impl std::ops::Deref for DeletedRequirements {
    type Target = Vec<Requirement>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for DeletedRequirements {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::fmt::Display for DeletedRequirements {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            writeln!(f, "No requirement was deleted.")
        } else {
            writeln!(f, "'{}' requirements deleted:", self.len())?;
            for req in &self.0 {
                writeln!(f, "- {}", req.id)?;
            }

            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequirementUpdate {
    pub old: Requirement,
    pub new: Requirement,
}

#[derive(Debug, Default, Clone)]
pub struct RequirementChanges {
    pub updated: Vec<RequirementUpdate>,
    pub inserted: Vec<Requirement>,
    pub unchanged_cnt: usize,
    pub new_generation: i64,
}

impl std::fmt::Display for RequirementChanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.updated.is_empty() && self.inserted.is_empty() {
            if self.unchanged_cnt == 0 {
                writeln!(f, "No requirements found.")?;
            } else {
                writeln!(f, "'{}' requirements kept.", self.unchanged_cnt)?;
            }
        } else {
            if !self.updated.is_empty() {
                writeln!(f, "'{}' requirements updated:", self.updated.len())?;
                for req in &self.updated {
                    writeln!(f, "- `{}`", req.new.id)?;
                }
            }

            if !self.inserted.is_empty() {
                writeln!(f, "'{}' requirements added:", self.inserted.len())?;
                for req in &self.inserted {
                    writeln!(f, "- `{}`", req.id)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum RequirementOrigin {
    GitHub(GitHubReqOrigin),
    Jira(String),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

    pub async fn add_reqs(&self, reqs: Vec<Requirement>) -> Result<RequirementChanges, DbError> {
        let mut changes = RequirementChanges::default();
        let old_generation = self.max_req_generation().await;
        let new_generation = old_generation + 1;
        changes.new_generation = new_generation;

        for req in &reqs {
            if let Ok(existing_record) = sqlx::query!(
                "select id, origin, annotation from Requirements where id = $1",
                req.id
            )
            .fetch_one(&self.pool)
            .await
            {
                let existing_req = Requirement {
                    id: existing_record.id,
                    origin: serde_json::from_str(&existing_record.origin)
                        .expect("Origin was serialized before."),
                    annotation: existing_record.annotation,
                };
                if req != &existing_req {
                    changes.updated.push(RequirementUpdate {
                        old: existing_req,
                        new: req.clone(),
                    });
                } else {
                    changes.unchanged_cnt += 1;
                }

                sqlx::query!(
                    "update Requirements set generation = $2, origin = $3, annotation = $4 where id = $1",
                    req.id,
                    new_generation,
                    req.origin,
                    req.annotation,
                )
                .execute(&self.pool)
                .await;
            } else {
                changes.inserted.push(req.clone());

                let res = sqlx::query!(
                    "insert into Requirements (id, generation, origin, annotation) values ($1, $2, $3, $4)",
                    req.id,
                    new_generation,
                    req.origin,
                    req.annotation,
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
        }

        for req in &changes.inserted {
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

        Ok(changes)
    }

    pub async fn delete_req_generations(
        &self,
        before: i64,
    ) -> Result<Option<DeletedRequirements>, DbError> {
        let mut deleted = DeletedRequirements::default();

        if let Ok(old_reqs) = sqlx::query!(
            "select id, origin, annotation from Requirements where generation < $1",
            before
        )
        .fetch_all(&self.pool)
        .await
        {
            for old_req in old_reqs {
                deleted.push(Requirement {
                    id: old_req.id,
                    origin: serde_json::from_str(&old_req.origin)
                        .expect("Origin was serialized before."),
                    annotation: old_req.annotation,
                })
            }
        }

        sqlx::query!("delete from Requirements where generation < $1", before)
            .execute(&self.pool)
            .await;

        Ok(if deleted.is_empty() {
            None
        } else {
            Some(deleted)
        })
    }

    pub async fn max_req_generation(&self) -> i64 {
        if let Ok(record) = sqlx::query!("select max(generation) as nr from Requirements")
            .fetch_one(&self.pool)
            .await
        {
            record.nr.unwrap_or_default()
        } else {
            0
        }
    }

    pub async fn reset_req_generation(&self) {
        sqlx::query!("update Requirements set generation = 0")
            .execute(&self.pool)
            .await;
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
        new_generation: i64,
    ) -> Result<TraceChanges, DbError> {
        let mut changes = TraceChanges::default();
        changes.new_generation = new_generation;

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
                if (sqlx::query!("select req_id, filepath, line from Traces where req_id = $1 and filepath = $2 and line = $3", id, file, line).fetch_one(&self.pool).await).is_ok() {
                    sqlx::query!("update Traces set generation = $4 where req_id = $1 and filepath = $2 and line = $3", id, file, line, new_generation).execute(&self.pool).await;
                    changes.unchanged_cnt += 1;
                } else {
                    changes.inserted.push(Trace{ req_id: id.clone(), filepath: PathBuf::from(&file), line });

                    let _ = sqlx::query!(
                        "insert into Traces (req_id, filepath, line, generation) values ($1, $2, $3, $4)",
                        id,
                        file,
                        line,
                        new_generation,
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
        }

        Ok(changes)
    }

    pub async fn max_trace_generation(&self) -> i64 {
        if let Ok(record) = sqlx::query!("select max(generation) as nr from Traces")
            .fetch_one(&self.pool)
            .await
        {
            record.nr.unwrap_or_default()
        } else {
            0
        }
    }

    pub async fn reset_trace_generation(&self) {
        sqlx::query!("update Traces set generation = 0")
            .execute(&self.pool)
            .await;
    }

    pub async fn delete_trace_generations(
        &self,
        before: i64,
    ) -> Result<Option<DeletedTraces>, DbError> {
        let mut deleted_traces = DeletedTraces::default();

        if let Ok(old_traces) = sqlx::query!(
            "select req_id, filepath, line from Traces where generation < $1",
            before
        )
        .fetch_all(&self.pool)
        .await
        {
            for old_trace in old_traces {
                deleted_traces.push(Trace {
                    req_id: old_trace.req_id,
                    filepath: PathBuf::from(old_trace.filepath),
                    line: old_trace.line.try_into().expect("Line must be u32."),
                })
            }
        }

        sqlx::query!("delete from Traces where generation < $1", before)
            .execute(&self.pool)
            .await;

        Ok(if deleted_traces.is_empty() {
            None
        } else {
            Some(deleted_traces)
        })
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

    pub async fn is_valid(&self) -> Result<(), DbError> {
        let traced_deprecated = sqlx::query!("select t.req_id from Traces as t, Requirements as r where t.req_id = r.id and r.annotation = 'deprecated' limit 100").fetch_all(&self.pool).await.map_err(|err| DbError::Validate(err.to_string()))?;

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
                let _ = sqlx::query!("delete from TestRuns")
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
