use std::path::{Path, PathBuf};

use mantra_lang_tracing::TraceEntry;
use mantra_schema::{
    coverage::{TestRunPk, TestState},
    requirements::Requirement,
    reviews::ReviewSchema,
    traces::TracePk,
};
use sqlx::Pool;

pub use sqlx;

use crate::cfg::{DeleteReqsConfig, DeleteReviewsConfig, DeleteTestRunsConfig, DeleteTracesConfig};

pub type DB = sqlx::sqlite::Sqlite;

#[derive(Debug)]
pub struct MantraDb {
    pool: Pool<DB>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeletedTraces(Vec<TracePk>);

impl std::ops::Deref for DeletedTraces {
    type Target = Vec<TracePk>;

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
    pub inserted: Vec<TracePk>,
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

#[derive(Debug, Clone, clap::Args)]
#[group(id = "db")]
pub struct Config {
    /// URL to connect to a SQL database.
    /// Default is a SQLite file named `mantra.db` that is located in the current directory.
    #[arg(long, alias = "db-url", env = "MANTRA_DB")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DbError {
    #[error("Could not get connection to database. Cause: {}", .0)]
    Connect(String),
    #[error("Could not run migration on database. Cause: {}", .0)]
    Migrate(String),
    #[error("Could not insert data into database. Cause: {}", .0)]
    Insert(String),
    #[error("Failed to delete table content. Cause: {}", .0)]
    Delete(String),
    #[error("Failed to update table content. Cause: {}", .0)]
    Update(String),
    #[error("The database contains invalid data. Cause: {}", .0)]
    Validate(String),
    #[error("{}", .0)]
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
            .map_err(|err| DbError::Connect(err.to_string()))?;

        MIGRATOR
            .run(&pool)
            .await
            .map_err(|err| DbError::Migrate(err.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn add_reqs(&self, reqs: Vec<Requirement>) -> Result<RequirementChanges, DbError> {
        let mut changes = RequirementChanges::default();
        let old_generation = self.max_req_generation().await;
        let new_generation = old_generation + 1;
        changes.new_generation = new_generation;

        for req in &reqs {
            if let Ok(existing_record) = sqlx::query!(
                "select id, title, link, info, manual, deprecated from Requirements where id = $1",
                req.id
            )
            .fetch_one(&self.pool)
            .await
            {
                let existing_req = Requirement {
                    id: existing_record.id,
                    title: existing_record.title,
                    link: existing_record.link,
                    info: existing_record.info.map(|a| {
                        serde_json::to_value(a).expect("Requirement info must be valid JSON.")
                    }),
                    manual: existing_record.manual,
                    deprecated: existing_record.deprecated,
                };
                if req != &existing_req {
                    changes.updated.push(RequirementUpdate {
                        old: existing_req,
                        new: req.clone(),
                    });
                } else {
                    changes.unchanged_cnt += 1;
                }

                let _ = sqlx::query!(
                    "update Requirements set generation = $2, title = $3, link = $4, info = $5, manual = $6, deprecated = $7 where id = $1",
                    req.id,
                    new_generation,
                    req.title,
                    req.link,
                    req.info,
                    req.manual,
                    req.deprecated,
                )
                .execute(&self.pool)
                .await;
            } else {
                let res = sqlx::query!(
                    "insert into Requirements (id, generation, title, link, info, manual, deprecated) values ($1, $2, $3, $4, $5, $6, $7)",
                    req.id,
                    new_generation,
                    req.title,
                    req.link,
                    req.info,
                    req.manual,
                    req.deprecated,
                )
                .execute(&self.pool)
                .await;

                if let Err(err) = res {
                    log::error!(
                        "Adding requirement '{}' failed with error: {}",
                        &req.id,
                        err
                    );
                } else {
                    changes.inserted.push(req.clone());
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
                        .ok_or(DbError::Insert(format!(
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
                    return Err(DbError::Insert(format!(
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
            "select id, title, link, info, manual, deprecated from Requirements where generation < $1",
            before
        )
        .fetch_all(&self.pool)
        .await
        {
            for old_req in old_reqs {
                deleted.push(Requirement {
                    id: old_req.id,
                    title: old_req.title,
                    link: old_req.link,
                    info: old_req.info.map(|a| serde_json::to_value(a)
                        .expect("Requirement info must be valid JSON.")),
                    manual: old_req.manual,
                    deprecated: old_req.deprecated,
                })
            }
        }

        let _ = sqlx::query!("delete from Requirements where generation < $1", before)
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
        let _ = sqlx::query!("update Requirements set generation = 0")
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
        traces: &[TraceEntry],
        new_generation: i64,
    ) -> Result<TraceChanges, DbError> {
        let mut changes = TraceChanges {
            new_generation,
            ..Default::default()
        };

        let file = filepath.display().to_string();

        for trace in traces {
            let ids = trace.ids();
            let line = trace.line();
            let line_span = trace.line_span();

            for id in ids {
                if (sqlx::query!("select req_id, filepath, line from Traces where req_id = $1 and filepath = $2 and line = $3", id, file, line).fetch_one(&self.pool).await).is_ok() {
                    let _ = sqlx::query!("update Traces set generation = $4 where req_id = $1 and filepath = $2 and line = $3", id, file, line, new_generation).execute(&self.pool).await;
                    changes.unchanged_cnt += 1;

                    if let Some(span) = line_span {
                        let start = span.start();
                        let end = span.end();

                        let _ = sqlx::query!("insert or replace into TraceSpans (req_id, filepath, line, start, end) values ($1, $2, $3, $4, $5)",
                            id,
                            file,
                            line,
                            start,
                            end,
                        ).execute(&self.pool).await;
                    }
                } else {
                    let res = sqlx::query!(
                        "insert into Traces (req_id, filepath, line, generation) values ($1, $2, $3, $4)",
                        id,
                        file,
                        line,
                        new_generation,
                    )
                    .execute(&self.pool)
                    .await;

                    if let Err(sqlx::Error::Database(err)) = res {
                        if err.kind() == sqlx::error::ErrorKind::ForeignKeyViolation {
                            log::warn!("Skipping trace. No requirement with id `{}` found for trace at file='{}', line='{}",
                                id, file, line);
                        } else {
                            log::error!("Adding trace for id=`{}`, file='{}', line='{}' failed with error: {}",
                                id, file, line, err);
                        }
                    } else {
                        changes.inserted.push(TracePk{ req_id: id.clone(), filepath: PathBuf::from(&file), line });

                        if let Some(span) = line_span {
                            let start = span.start();
                            let end = span.end();

                            let _ = sqlx::query!("insert into TraceSpans (req_id, filepath, line, start, end) values ($1, $2, $3, $4, $5)",
                                id,
                                file,
                                line,
                                start,
                                end,
                            ).execute(&self.pool).await;
                        }
                    }
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
        let _ = sqlx::query!("update Traces set generation = 0")
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
                deleted_traces.push(TracePk {
                    req_id: old_trace.req_id,
                    filepath: PathBuf::from(old_trace.filepath),
                    line: old_trace.line.try_into().expect("Line must be u32."),
                })
            }
        }

        let _ = sqlx::query!("delete from Traces where generation < $1", before)
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
        test_run: &TestRunPk,
        test_name: &str,
        trace_filepath: &Path,
        trace_line: u32,
        req_id: &str,
    ) -> Result<(), DbError> {
        // Note: filepath is already *fix* due to how the "file!()" macro works
        let file = trace_filepath.display().to_string();
        let query_result = sqlx::query!(
                "insert or ignore into TestCoverage (req_id, test_run_name, test_run_date, test_name, trace_filepath, trace_line) values ($1, $2, $3, $4, $5, $6)",
                req_id,
                test_run.name,
                test_run.date,
                test_name,
                file,
                trace_line,
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
                DbError::Insert(format!(
                    "Adding coverage for id='{}', test-run='{}' at {}, test='{}', file='{}', line='{}' failed with error: {}",
                    req_id, test_run.name, test_run.date, test_name, file, trace_line, err
                ))
            })?;

        Ok(())
    }

    pub async fn add_test(
        &self,
        test_run: &TestRunPk,
        name: &str,
        filepath: &Path,
        line: u32,
        state: TestState,
    ) -> Result<(), DbError> {
        let file = filepath.display().to_string();

        match state {
            TestState::Passed | TestState::Failed => {
                let passed = state == TestState::Passed;

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
                    DbError::Insert(format!(
                        "Adding test for test='{}', test-run='{}' at {}, file='{}', line='{}' failed with error: {}",
                        name, test_run.name, test_run.date, file, line, err
                    ))
                })?;
            }
            TestState::Skipped { reason } => {
                let file = filepath.display().to_string();
                sqlx::query!(
                        "insert or ignore into SkippedTests (name, test_run_name, test_run_date, filepath, line, reason) values ($1, $2, $3, $4, $5, $6)",
                        name,
                        test_run.name,
                        test_run.date,
                        file,
                        line,
                        reason,
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|err| {
                        DbError::Insert(format!(
                            "Adding skipped test '{}' for test-run='{}' at {}, file='{}', line='{}' failed with error: {}",
                            name, test_run.name, test_run.date, file, line, err
                        ))
                    })?;
            }
        }

        Ok(())
    }

    // pub async fn add_skipped_test(
    //     &self,
    //     test_run: &TestRunConfig,
    //     name: &str,
    //     filepath: &Path,
    //     line: u32,
    //     reason: Option<String>,
    // ) -> Result<(), DbError> {
    //     let file = filepath.display().to_string();
    //     sqlx::query!(
    //             "insert or ignore into SkippedTests (name, test_run_name, test_run_date, filepath, line, reason) values ($1, $2, $3, $4, $5, $6)",
    //             name,
    //             test_run.name,
    //             test_run.date,
    //             file,
    //             line,
    //             reason,
    //         )
    //         .execute(&self.pool)
    //         .await
    //         .map_err(|err| {
    //             DbError::Insert(format!(
    //                 "Adding skipped test '{}' for test-run='{}' at {}, file='{}', line='{}' failed with error: {}",
    //                 name, test_run.name, test_run.date, file, line, err
    //             ))
    //         })?;

    //     Ok(())
    // }

    pub async fn add_test_run(
        &self,
        name: &str,
        date: &time::OffsetDateTime,
        nr_of_tests: u32,
        meta: Option<serde_json::Value>,
        logs: Option<String>,
    ) -> Result<(), DbError> {
        let _ = sqlx::query!(
            "insert or ignore into TestRuns (name, date, nr_of_tests, meta, logs) values ($1, $2, $3, $4, $5)",
            name,
            date,
            nr_of_tests,
            meta,
            logs,
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            DbError::Insert(format!(
                "Adding test-run with name='{}' and date='{}' failed with error: {}",
                name, date, err
            ))
        })?;

        Ok(())
    }

    // pub async fn update_nr_of_tests(
    //     &self,
    //     test_run: &TestRunConfig,
    //     nr_of_tests: u32,
    // ) -> Result<(), DbError> {
    //     let _ = sqlx::query!(
    //         "update TestRuns set nr_of_tests = $1 where name = $2 and date = $3 and nr_of_tests is null",
    //         nr_of_tests,
    //         test_run.name,
    //         test_run.date,
    //     )
    //     .execute(&self.pool)
    //     .await
    //     .map_err(|err| {
    //         DbError::Update(format!(
    //             "Could not set 'nr_of_tests' for test-run='{}' at {}. Cause: {}",
    //             test_run.name, test_run.date, err
    //         ))
    //     })?;

    //     Ok(())
    // }

    pub async fn is_valid(&self) -> Result<(), DbError> {
        let record = sqlx::query!("select count(*) as invalid_cnt from InvalidRequirements")
            .fetch_one(&self.pool)
            .await
            .map_err(|err| DbError::Validate(err.to_string()))?;

        if record.invalid_cnt == 0 {
            Ok(())
        } else {
            Err(DbError::Validate(format!(
                "'{}' deprecated requirements have trace entries.",
                record.invalid_cnt
            )))
        }
    }

    pub fn pool(&self) -> &Pool<DB> {
        // workaround for custom queries
        &self.pool
    }

    pub async fn delete_old_generations(&self, clean: bool) -> Result<(), DbError> {
        let _ = sqlx::query!(
            "delete from Requirements where generation < (select max(generation) from Requirements)"
        )
        .execute(&self.pool)
        .await
        .map_err(|err| DbError::Delete(err.to_string()))?;
        let _ = sqlx::query!(
            "delete from Traces where generation < (select max(generation) from Traces)"
        )
        .execute(&self.pool)
        .await
        .map_err(|err| DbError::Delete(err.to_string()))?;

        if clean {
            self.clean().await?;
        }

        Ok(())
    }

    pub async fn delete_reqs(&self, cfg: &DeleteReqsConfig) -> Result<(), DbError> {
        let ids = cfg.ids.as_deref().unwrap_or_default();

        if ids.is_empty() {
            if let Some(before) = cfg.before {
                let _ = sqlx::query!("delete from Requirements where generation < $1", before)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        } else {
            for id in ids {
                match cfg.before {
                    Some(before) => {
                        sqlx::query!(
                            "delete from Requirements where id = $1 and generation < $2",
                            id,
                            before
                        )
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;
                    }
                    None => {
                        sqlx::query!("delete from Requirements where id = $1", id)
                            .execute(&self.pool)
                            .await
                            .map_err(|err| DbError::Delete(err.to_string()))?;
                    }
                };
            }
        }

        Ok(())
    }

    pub async fn delete_traces(&self, cfg: &DeleteTracesConfig) -> Result<(), DbError> {
        let ids = cfg.req_ids.as_deref().unwrap_or_default();

        if ids.is_empty() {
            if let Some(before) = cfg.before {
                let _ = sqlx::query!("delete from Traces where generation < $1", before)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        } else {
            for id in ids {
                match cfg.before {
                    Some(before) => {
                        sqlx::query!(
                            "delete from Traces where req_id = $1 and generation < $2",
                            id,
                            before
                        )
                        .execute(&self.pool)
                        .await
                        .map_err(|err| DbError::Delete(err.to_string()))?;
                    }
                    None => {
                        sqlx::query!("delete from Traces where req_id = $1", id)
                            .execute(&self.pool)
                            .await
                            .map_err(|err| DbError::Delete(err.to_string()))?;
                    }
                };
            }
        }

        Ok(())
    }

    pub async fn delete_test_runs(&self, cfg: DeleteTestRunsConfig) -> Result<(), DbError> {
        match cfg.before {
            Some(before) => {
                sqlx::query!(
                    "delete from TestRuns where unixepoch(date) < unixepoch($1)",
                    before
                )
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            }
            None => {
                sqlx::query!("delete from TestRuns")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        }

        Ok(())
    }

    pub async fn add_review(&self, review: ReviewSchema) -> Result<(), DbError> {
        sqlx::query!(
            "insert or replace into Reviews (name, date, reviewer, comment) values ($1, $2, $3, $4)",
            review.name,
            review.date,
            review.reviewer,
            review.comment,
        )
        .execute(&self.pool)
        .await
        .map_err(|err| DbError::Insert(err.to_string()))?;

        for req in review.requirements {
            let res = sqlx::query!(
                "insert or replace into ManuallyVerified (req_id, review_name, review_date, comment) values ($1, $2, $3, $4)",
                req.id,
                review.name,
                review.date,
                req.comment,
            )
            .execute(&self.pool)
            .await;

            if let Err(sqlx::Error::Database(err)) = res {
                if err.kind() == sqlx::error::ErrorKind::ForeignKeyViolation {
                    log::error!(
                        "Requirement '{}' in review '{}' not in database.",
                        req.id,
                        review.name
                    );
                } else {
                    log::error!(
                        "Failed to insert requirement '{}' from review '{}'. Cause: {}",
                        req.id,
                        review.name,
                        err
                    );
                }
            }
        }

        Ok(())
    }

    pub async fn delete_reviews(&self, cfg: DeleteReviewsConfig) -> Result<(), DbError> {
        match cfg.before {
            Some(before) => {
                sqlx::query!(
                    "delete from Reviews where unixepoch(date) < unixepoch($1)",
                    before
                )
                .execute(&self.pool)
                .await
                .map_err(|err| DbError::Delete(err.to_string()))?;
            }
            None => {
                sqlx::query!("delete from Reviews")
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DbError::Delete(err.to_string()))?;
            }
        }

        Ok(())
    }

    pub async fn clean(&self) -> Result<(), DbError> {
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

        Ok(())
    }
}
