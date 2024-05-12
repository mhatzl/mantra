use std::path::PathBuf;

use time::OffsetDateTime;

use crate::db::{MantraDb, RequirementOrigin};

use super::coverage::iso8601_str_to_offsetdatetime;

#[derive(Debug)]
pub enum ReportError {
    Db(sqlx::Error),
    Serialize,
    Tera,
    Format,
    Write,
    Template,
}

#[derive(Debug, Clone, clap::Args)]
pub struct ReportConfig {
    pub path: PathBuf,
    #[arg(long)]
    pub template: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}

pub async fn report(db: &MantraDb, cfg: ReportConfig) -> Result<(), ReportError> {
    let mut filepath = if cfg.path.is_file() {
        cfg.path
    } else {
        let now = OffsetDateTime::now_utc();
        let format =
            time::macros::format_description!("[year][month][day]_[hour]h[minute]m[second]s");
        let filename = format!(
            "{}_mantra_report",
            now.format(format).map_err(|_| ReportError::Format)?
        );
        cfg.path.join(filename)
    };

    let report = if cfg.json {
        filepath.set_extension("json");
        create_json_report(db).await?
    } else {
        let template_content = match &cfg.template {
            Some(template) => {
                std::fs::read_to_string(template).map_err(|_| ReportError::Template)?
            }
            None => include_str!("report_default_template.html").to_string(),
        };

        create_tera_report(db, &template_content).await?
    };

    std::fs::write(filepath, report).map_err(|_| ReportError::Write)
}

pub async fn create_tera_report(db: &MantraDb, template: &str) -> Result<String, ReportError> {
    let context = tera::Context::from_serialize(ReportContext::try_from(db).await?)
        .map_err(|_| ReportError::Tera)?;
    tera::Tera::one_off(template, &context, true).map_err(|_| ReportError::Tera)
}

pub async fn create_json_report(db: &MantraDb) -> Result<String, ReportError> {
    let report = ReportContext::try_from(db).await?;
    serde_json::to_string_pretty(&report).map_err(|_| ReportError::Serialize)
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ReportContext {
    pub overview: RequirementsOverview,
    pub requirements: Vec<RequirementInfo>,
    pub tests: TestStatistics,
    pub reviews: Vec<Review>,
    pub trace_criteria: &'static str,
    pub test_coverage_criteria: &'static str,
    pub test_passed_coverage_criteria: &'static str,
}

impl ReportContext {
    pub async fn try_from(db: &MantraDb) -> Result<Self, ReportError> {
        let overview = RequirementsOverview::try_from(db).await?;

        let req_records = sqlx::query!("select id from Requirements")
            .fetch_all(db.pool())
            .await
            .map_err(ReportError::Db)?;

        let mut requirements = Vec::new();
        for req in req_records {
            requirements.push(RequirementInfo::try_from(db, req.id).await?);
        }

        let tests = TestStatistics::try_from(db).await?;

        let review_records = sqlx::query!("select name, date from Reviews")
            .fetch_all(db.pool())
            .await
            .map_err(ReportError::Db)?;

        let mut reviews = Vec::new();
        for review in review_records {
            let date = OffsetDateTime::parse(
                &review.date,
                &time::format_description::well_known::Iso8601::DEFAULT,
            )
            .expect("Test run date was added to db in ISO8601 format.");
            reviews.push(Review::try_from(db, review.name, date).await?);
        }

        let trace_criteria = "
Requirements are traced if one of the following criterias is met:

- A trace directly referring to the requirement exists (Directly traced)
- All of the leaf requirements of the requirement have direct traces (Indirectly traced)
";

        let test_coverage_criteria = "
A requirement is covered through a test if any of the following criterias are met:

- At least one direct trace to the requirement was reached during test execution
- All leaf requirements of the requirement were covered by the test
";

        let test_passed_coverage_criteria = "
A requirement coverage passed if all of the following criterias are met:

- All tests covering the requirement passed
- All tests covering the child requirements of the requirement passed
";

        Ok(Self {
            overview,
            requirements,
            tests,
            reviews,
            trace_criteria,
            test_coverage_criteria,
            test_passed_coverage_criteria,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct RequirementsOverview {
    pub req_cnt: i32,
    pub traced_cnt: i32,
    pub traced_ratio: f64,
    pub covered_cnt: i32,
    pub covered_ratio: f64,
    pub passed_cnt: i32,
    pub passed_ratio: f64,
}

impl RequirementsOverview {
    pub async fn try_from(db: &MantraDb) -> Result<Self, ReportError> {
        let record = sqlx::query!("select * from RequirementCoverageOverview")
            .fetch_one(db.pool())
            .await
            .map_err(ReportError::Db)?;

        Ok(Self {
            req_cnt: record.req_cnt.unwrap_or_default(),
            traced_cnt: record.traced_cnt.unwrap_or_default(),
            traced_ratio: record.traced_ratio.unwrap_or_default(),
            covered_cnt: record.covered_cnt.unwrap_or_default(),
            covered_ratio: record.covered_ratio.unwrap_or_default(),
            passed_cnt: record.passed_cnt.unwrap_or_default(),
            passed_ratio: record.passed_ratio.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RequirementInfo {
    pub id: String,
    pub origin: RequirementOrigin,
    pub annotation: Option<String>,
    pub deprecated: bool,
    pub manual: bool,
    pub trace_info: RequirementTraceInfo,
    pub test_coverage_info: RequirementTestCoverageInfo,
    pub review_info: RequirementReviewInfo,
}

impl RequirementInfo {
    pub async fn try_from(db: &MantraDb, id: impl Into<String>) -> Result<Self, ReportError> {
        let id: String = id.into();

        // get base info
        let record = sqlx::query!(r#"
            select 
                origin,
                annotation,
                case when id in (select id from DeprecatedRequirements) then true else false end as "deprecated!: bool",
                case when id in (select id from ManualRequirements) then true else false end as "manual!: bool"
            from Requirements
            where id = $1
        "#, id).fetch_one(db.pool()).await.map_err(ReportError::Db)?;

        let origin = serde_json::from_str(&record.origin).expect("Origin was serialized into db.");
        let annotation = record.annotation;
        let deprecated = record.deprecated;
        let manual = record.manual;

        let trace_info = RequirementTraceInfo::try_from(db, &id).await?;
        let test_coverage_info = RequirementTestCoverageInfo::try_from(db, &id).await?;
        let review_info = RequirementReviewInfo::try_from(db, &id).await?;

        Ok(Self {
            id,
            origin,
            annotation,
            deprecated,
            manual,
            trace_info,
            test_coverage_info,
            review_info,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RequirementTraceInfo {
    pub traced: bool,
    pub direct_traces: Vec<DirectTraceInfo>,
    pub indirect_traces: Vec<IndirectTraceInfo>,
}

impl RequirementTraceInfo {
    pub async fn try_from(db: &MantraDb, id: &str) -> Result<Self, ReportError> {
        let records = sqlx::query_as!(
            DirectTraceInfo,
            r#"
            select filepath, line as "line: u32"
            from Traces
            where req_id = $1
        "#,
            id
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let direct_traces = records;

        let records = sqlx::query_as!(
            IndirectTraceInfo,
            r#"
            select traced_id as "traced_id!", filepath, line as "line!: u32"
            from IndirectRequirementTraces
            where id = $1
        "#,
            id
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let indirect_traces = records;

        Ok(Self {
            traced: !direct_traces.is_empty() || !indirect_traces.is_empty(),
            direct_traces,
            indirect_traces,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DirectTraceInfo {
    pub filepath: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct IndirectTraceInfo {
    pub traced_id: String,
    pub filepath: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RequirementTestCoverageInfo {
    pub covered: bool,
    pub passed: bool,
    pub direct_coverage: Vec<DirectTestCoverageInfo>,
    pub indirect_coverage: Vec<IndirectTestCoverageInfo>,
}

impl RequirementTestCoverageInfo {
    pub async fn try_from(db: &MantraDb, id: &str) -> Result<Self, ReportError> {
        let records = sqlx::query!(
            r#"
            select test_run_name, test_run_date, test_name, filepath, line as "line: u32"
            from TestCoverage
            where req_id = $1
        "#,
            id
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let mut direct_coverage = Vec::with_capacity(records.len());

        for record in records {
            direct_coverage.push(DirectTestCoverageInfo {
                test_run_name: record.test_run_name,
                test_run_date: iso8601_str_to_offsetdatetime(&record.test_run_date),
                test_name: record.test_name,
                filepath: record.filepath,
                line: record.line,
            })
        }

        let records = sqlx::query!(
            r#"
            select covered_id as "covered_id!", test_run_name, test_run_date, test_name, filepath, line as "line!: u32"
            from IndirectRequirementTestCoverage
            where id = $1
        "#,
            id
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let mut indirect_coverage = Vec::with_capacity(records.len());

        for record in records {
            indirect_coverage.push(IndirectTestCoverageInfo {
                covered_id: record.covered_id,
                test_run_name: record.test_run_name,
                test_run_date: iso8601_str_to_offsetdatetime(&record.test_run_date),
                test_name: record.test_name,
                filepath: record.filepath,
                line: record.line,
            })
        }

        let passed = sqlx::query!(
            "
            select count(*) as cnt
            from PassedCoveredRequirements
            where id = $1
            ",
            id
        )
        .fetch_one(db.pool())
        .await
        .ok()
        .is_some();

        Ok(Self {
            covered: !direct_coverage.is_empty() || !indirect_coverage.is_empty(),
            passed,
            direct_coverage,
            indirect_coverage,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DirectTestCoverageInfo {
    pub test_run_name: String,
    pub test_run_date: OffsetDateTime,
    pub test_name: String,
    pub filepath: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct IndirectTestCoverageInfo {
    pub covered_id: String,
    pub test_run_name: String,
    pub test_run_date: OffsetDateTime,
    pub test_name: String,
    pub filepath: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RequirementReviewInfo {
    pub verified: bool,
    pub review_name: String,
    pub review_date: String,
    pub comment: Option<String>,
}

impl RequirementReviewInfo {
    pub async fn try_from(db: &MantraDb, id: &str) -> Result<Self, ReportError> {
        sqlx::query_as!(
            RequirementReviewInfo,
            r#"
            select true as "verified: bool", review_name, review_date, comment
            from ManuallyVerified
            where req_id = $1
        "#,
            id
        )
        .fetch_one(db.pool())
        .await
        .map_err(ReportError::Db)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct TestStatistics {
    pub overview: TestsOverview,
    pub test_runs: Vec<TestRunOverview>,
}

impl TestStatistics {
    pub async fn try_from(db: &MantraDb) -> Result<Self, ReportError> {
        let overview = TestsOverview::try_from(db).await?;

        let test_run_records = sqlx::query!(
            "
            select name, date
            from TestRuns
            "
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let mut test_runs = Vec::new();

        for test_run in test_run_records {
            let date = iso8601_str_to_offsetdatetime(&test_run.date);
            test_runs.push(TestRunOverview::try_from(db, &test_run.name, &date).await?);
        }

        Ok(Self {
            overview,
            test_runs,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct TestsOverview {
    pub test_cnt: i32,
    pub ran_cnt: i32,
    pub ran_ratio: f64,
    pub passed_cnt: i32,
    pub passed_ratio: f64,
    pub failed_cnt: i32,
    pub failed_ratio: f64,
    pub skipped_cnt: i32,
    pub skipped_ratio: f64,
}

impl TestsOverview {
    pub async fn try_from(db: &MantraDb) -> Result<Self, ReportError> {
        sqlx::query_as!(
            TestsOverview,
            r#"
                select 
                test_cnt as "test_cnt!: i32",
                ran_cnt as "ran_cnt!: i32",
                ran_ratio as "ran_ratio!: f64",
                passed_cnt as "passed_cnt!: i32",
                passed_ratio as "passed_ratio!: f64",
                failed_cnt as "failed_cnt!: i32",
                failed_ratio as "failed_ratio!: f64",
                skipped_cnt as "skipped_cnt!: i32",
                skipped_ratio as "skipped_ratio!: f64"
                from OverallTestOverview
                "#
        )
        .fetch_one(db.pool())
        .await
        .map_err(ReportError::Db)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct TestRunInfo {
    pub overview: TestRunOverview,
    pub name: String,
    pub date: OffsetDateTime,
    pub logs: Option<String>,
    pub tests: Vec<TestInfo>,
}

impl TestRunInfo {
    pub async fn try_from(
        db: &MantraDb,
        name: impl Into<String>,
        date: OffsetDateTime,
    ) -> Result<Self, ReportError> {
        let name: String = name.into();
        let overview = TestRunOverview::try_from(db, &name, &date).await?;

        let test_records = sqlx::query!(
            r#"
            select
            name, filepath, line as "line: u32",
            passed as "passed!: bool",
            false as "skipped!: bool",
            null as "reason?: String"
            from Tests
            where test_run_name = $1 and test_run_date = $2
            
            union all
            
            select
            name, filepath, line as "line: u32",
            false as "passed!: bool",
            true as "skipped!: bool",
            reason
            from SkippedTests
            where test_run_name = $1 and test_run_date = $2
        "#,
            name,
            date
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let mut test_infos = Vec::new();

        for test in test_records {
            let covers = sqlx::query!(
                "
                select req_id from TestCoverage
                where test_run_name = $1 and
                test_run_date = $2 and
                test_name = $3
                ",
                name,
                date,
                test.name
            )
            .fetch_all(db.pool())
            .await
            .map_err(ReportError::Db)?
            .into_iter()
            .map(|r| r.req_id)
            .collect();

            let state = if test.passed {
                TestState::Passed
            } else if test.skipped {
                TestState::Skipped {
                    reason: test.reason,
                }
            } else {
                TestState::Failed
            };

            test_infos.push(TestInfo {
                covers,
                name: test.name,
                filepath: PathBuf::from(test.filepath),
                line: test.line,
                state,
            })
        }

        let record = sqlx::query!(
            r#"
            select logs from TestRuns
            where name = $1 and date = $2
            "#,
            name,
            date
        )
        .fetch_one(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let logs = record.logs;

        Ok(Self {
            overview,
            name,
            date,
            logs,
            tests: test_infos,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct TestRunOverview {
    pub test_cnt: i64,
    pub ran_cnt: i64,
    pub ran_ratio: f64,
    pub passed_cnt: i64,
    pub passed_ratio: f64,
    pub failed_cnt: i64,
    pub failed_ratio: f64,
    pub skipped_cnt: i64,
    pub skipped_ratio: f64,
}

impl TestRunOverview {
    pub async fn try_from(
        db: &MantraDb,
        name: &str,
        date: &OffsetDateTime,
    ) -> Result<Self, ReportError> {
        sqlx::query_as!(
            TestRunOverview,
            r#"
                select 
                test_cnt as "test_cnt!: i32",
                ran_cnt as "ran_cnt!: i32",
                ran_ratio as "ran_ratio!: f64",
                passed_cnt as "passed_cnt!: i32",
                passed_ratio as "passed_ratio!: f64",
                failed_cnt as "failed_cnt!: i32",
                failed_ratio as "failed_ratio!: f64",
                skipped_cnt as "skipped_cnt!: i32",
                skipped_ratio as "skipped_ratio!: f64"
                from TestRunOverview
                where name = $1 and date = $2
                "#,
            name,
            date
        )
        .fetch_one(db.pool())
        .await
        .map_err(ReportError::Db)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct TestInfo {
    /// List of requirements that are covered by this test.
    pub covers: Vec<String>,
    pub name: String,
    pub filepath: PathBuf,
    pub line: u32,
    pub state: TestState,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum TestState {
    Passed,
    Failed,
    Skipped { reason: Option<String> },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Review {
    pub name: String,
    pub date: OffsetDateTime,
    pub reviewer: String,
    pub comment: Option<String>,
    pub verified_reqs: Vec<VerifiedRequirement>,
}

impl Review {
    pub async fn try_from(
        db: &MantraDb,
        name: impl Into<String>,
        date: OffsetDateTime,
    ) -> Result<Self, ReportError> {
        let name: String = name.into();

        let records = sqlx::query_as!(
            VerifiedRequirement,
            "
                select req_id as id, comment
                from ManuallyVerified
                where review_name = $1 and review_date = $2
            ",
            name,
            date
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let verified_reqs = records;

        let record = sqlx::query!(
            "
                select reviewer, comment
                from Reviews
                where name = $1 and date = $2
            ",
            name,
            date
        )
        .fetch_one(db.pool())
        .await
        .map_err(ReportError::Db)?;

        Ok(Review {
            name,
            date,
            reviewer: record.reviewer,
            comment: record.comment,
            verified_reqs,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct VerifiedRequirement {
    pub id: String,
    pub comment: Option<String>,
}