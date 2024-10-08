use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use mantra_schema::{
    coverage::TestState,
    requirements::{ReqId, Requirement},
    traces::TracePk,
    Line,
};
use time::{OffsetDateTime, PrimitiveDateTime};

use crate::{cmd::review::VerifiedRequirement, db::MantraDb};

use super::{coverage::iso8601_str_to_offsetdatetime, review::Review};

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("{}", .0)]
    Db(sqlx::Error),
    #[error("Failed to serialize report data.")]
    Serialize,
    #[error("Failed to generate the report. Make sure the report template is valid.")]
    Tera,
    #[error("Failed to format date/time for the report name.")]
    Format,
    #[error("Failed to write the report.")]
    Write,
    #[error("Failed to read the given template.")]
    Template,
}

#[derive(Debug, Clone, clap::Args)]
pub struct ReportConfig {
    pub path: PathBuf,
    #[arg(long)]
    pub template: Option<PathBuf>,
    #[arg(long)]
    pub formats: Vec<ReportFormat>,
    #[command(flatten)]
    pub project: Project,
    #[command(flatten)]
    pub tag: Tag,
    /// Path to a Tera template that is used to render the custom info of requirements.
    #[arg(long)]
    pub info_template: Option<PathBuf>,
    /// Path to a Tera template that is used to render the custom metadata of test-runs.
    #[arg(long)]
    pub test_run_template: Option<PathBuf>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, clap::Args, schemars::JsonSchema,
)]
pub struct Project {
    #[arg(id = "project-name", long = "project-name")]
    pub name: Option<String>,
    #[arg(id = "project-version", long = "project-version")]
    pub version: Option<String>,
    #[arg(id = "project-link", long = "project-link")]
    pub link: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, clap::Args, schemars::JsonSchema,
)]
pub struct Tag {
    #[arg(id = "tag-name", long = "tag-name")]
    pub name: Option<String>,
    #[arg(id = "tag-link", long = "tag-link")]
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum ReportFormat {
    Html,
    Json,
}

pub async fn report(db: &MantraDb, cfg: ReportConfig) -> Result<(), ReportError> {
    let mut filepath = if cfg.path.extension().is_some() {
        cfg.path
    } else {
        let now = OffsetDateTime::now_utc();
        let format =
            time::macros::format_description!("[year][month][day]_[hour]h[minute]m[second]s");
        let filename = format!(
            "{}_mantra_report.html",
            now.format(format).map_err(|_| ReportError::Format)?
        );
        cfg.path.join(filename)
    };

    let formats: HashSet<ReportFormat> = HashSet::from_iter(cfg.formats.into_iter());

    for format in formats {
        let report = match format {
            ReportFormat::Html => {
                filepath.set_extension("html");

                let template_content = match &cfg.template {
                    Some(template) => tokio::fs::read_to_string(template)
                        .await
                        .map_err(|_| ReportError::Template)?,
                    None => include_str!("report_default_template.html").to_string(),
                };

                create_tera_report(
                    db,
                    &cfg.project,
                    &cfg.tag,
                    cfg.info_template.as_deref(),
                    cfg.test_run_template.as_deref(),
                    &template_content,
                )
                .await?
            }
            ReportFormat::Json => {
                filepath.set_extension("json");

                create_json_report(
                    db,
                    &cfg.project,
                    &cfg.tag,
                    cfg.info_template.as_deref(),
                    cfg.test_run_template.as_deref(),
                )
                .await?
            }
        };

        tokio::fs::write(&filepath, report)
            .await
            .map_err(|_| ReportError::Write)?;
    }

    Ok(())
}

pub async fn create_tera_report(
    db: &MantraDb,
    project: &Project,
    tag: &Tag,
    info_template: Option<&Path>,
    test_run_template: Option<&Path>,
    template: &str,
) -> Result<String, ReportError> {
    let context = tera::Context::from_serialize(
        ReportContext::try_from(db, project, tag, info_template, test_run_template).await?,
    )
    .map_err(|_| ReportError::Tera)?;
    tera::Tera::one_off(template, &context, true).map_err(|_| ReportError::Tera)
}

pub async fn create_json_report(
    db: &MantraDb,
    project: &Project,
    tag: &Tag,
    info_template: Option<&Path>,
    test_run_template: Option<&Path>,
) -> Result<String, ReportError> {
    let report =
        ReportContext::try_from(db, project, tag, info_template, test_run_template).await?;
    serde_json::to_string_pretty(&report).map_err(|_| ReportError::Serialize)
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ReportContext {
    pub project: Project,
    pub tag: Tag,
    pub overview: RequirementsOverview,
    pub requirements: Vec<RequirementInfo>,
    pub tests: TestStatistics,
    pub reviews: Vec<Review>,
    pub trace_criteria: &'static str,
    pub test_coverage_criteria: &'static str,
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    #[schemars(with = "String")]
    pub creation_date: OffsetDateTime,
    pub validation: ValidationInfo,
    pub unrelated: Unrelated,
}

impl ReportContext {
    pub async fn try_from(
        db: &MantraDb,
        project: &Project,
        tag: &Tag,
        info_template: Option<&Path>,
        test_run_template: Option<&Path>,
    ) -> Result<Self, ReportError> {
        let overview = RequirementsOverview::try_from(db).await?;

        let req_records = sqlx::query!("select id from Requirements order by id")
            .fetch_all(db.pool())
            .await
            .map_err(ReportError::Db)?;

        let mut requirements = Vec::new();
        for req in req_records {
            requirements.push(RequirementInfo::try_from(db, req.id, info_template).await?);
        }

        let tests = TestStatistics::try_from(db, test_run_template).await?;

        let review_records = sqlx::query!("select name, date from Reviews order by name, date")
            .fetch_all(db.pool())
            .await
            .map_err(ReportError::Db)?;

        let mut reviews = Vec::new();
        for review in review_records {
            let date = PrimitiveDateTime::parse(&review.date, &super::REVIEW_DATE_FORMAT)
                .expect("Review date was added to db in custom review-date format.");
            reviews.push(Review::try_from(db, review.name, date).await?);
        }

        let trace_criteria = "Requirements are traced if one of the following criteria is met:

- A trace directly referring to the requirement exists (Directly traced)
- All of the sub-requirements of the requirement are traced (Indirectly traced)";

        let test_coverage_criteria =
            "Requirements are covered through a test if one of the following criteria is met:

- At least one direct trace to the requirement was reached during test execution (Direct coverage)
- All sub-requirements of the requirement were covered by the test (Indirect coverage)

Requirements are passed if all of the following criteria are met:

- The requirement is covered at least once
- All tests covering the requirement passed
- All tests covering child requirements of the requirement passed";

        let creation_date = OffsetDateTime::now_utc();

        let validation = ValidationInfo::try_from(db).await?;

        let unrelated = Unrelated::try_from(db).await?;

        Ok(Self {
            project: project.clone(),
            tag: tag.clone(),
            overview,
            requirements,
            tests,
            reviews,
            trace_criteria,
            test_coverage_criteria,
            creation_date,
            validation,
            unrelated,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ValidationInfo {
    pub is_valid: bool,
    pub criteria: &'static str,
    pub invalid_reqs: Vec<String>,
}

impl ValidationInfo {
    pub async fn try_from(db: &MantraDb) -> Result<Self, ReportError> {
        let validation_criteria =
            "The collected data is valid if no *deprecated* requirement is traced.";
        let is_valid = db.is_valid().await.is_ok();

        if is_valid {
            Ok(Self {
                is_valid,
                criteria: validation_criteria,
                invalid_reqs: vec![],
            })
        } else {
            let invalid_records = sqlx::query!(r#"select id as "id!" from InvalidRequirements"#)
                .fetch_all(db.pool())
                .await
                .map_err(ReportError::Db)?;
            let invalid_reqs = invalid_records.into_iter().map(|r| r.id).collect();
            Ok(Self {
                is_valid,
                criteria: validation_criteria,
                invalid_reqs,
            })
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct RequirementsOverview {
    pub req_cnt: i32,
    pub traced_cnt: i32,
    pub traced_ratio: f64,
    pub covered_cnt: i32,
    pub covered_ratio: f64,
    pub passed_cnt: i32,
    pub passed_ratio: f64,
    pub verified_cnt: Option<i32>,
    pub verified_ratio: f64,
}

impl RequirementsOverview {
    pub async fn try_from(db: &MantraDb) -> Result<Self, ReportError> {
        let record = sqlx::query!(
            r#"select
                req_cnt,
                traced_cnt,
                traced_ratio,
                covered_cnt,
                covered_ratio,
                passed_cnt,
                passed_ratio,
                verified_cnt as "verified_cnt?: i32",
                verified_ratio
             from RequirementCoverageOverview"#
        )
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
            verified_cnt: record.verified_cnt,
            verified_ratio: record.verified_ratio.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementInfo {
    #[serde(flatten)]
    pub meta: Requirement,
    pub rendered_info: Option<String>,
    pub parent: Option<String>,
    pub direct_children: Vec<String>,
    pub leaf_statistic: Option<LeafChildrenStatistic>,
    pub trace_info: RequirementTraceInfo,
    pub test_coverage_info: RequirementTestCoverageInfo,
    pub verified_info: Vec<VerifiedRequirementInfo>,
    pub valid: bool,
}

impl RequirementInfo {
    pub async fn try_from(
        db: &MantraDb,
        id: impl Into<ReqId>,
        info_template: Option<&Path>,
    ) -> Result<Self, ReportError> {
        let id: ReqId = id.into();

        // get base info
        let record = sqlx::query!(r#"
            select 
                title,
                link,
                info,
                case when id in (select id from DeprecatedRequirements) then true else false end as "deprecated!: bool",
                case when id in (select id from ManualRequirements) then true else false end as "manual!: bool"
            from Requirements
            where id = $1
        "#, id).fetch_one(db.pool()).await.map_err(ReportError::Db)?;

        let title = record.title;
        let link = record.link;
        let info = record
            .info
            .map(|a| serde_json::from_str(&a).expect("Requirement info must be valid JSON."));
        let deprecated = record.deprecated;
        let manual = record.manual;

        let record = sqlx::query!(
            r#"
                select parent_id
                from RequirementHierarchies
                where child_id = $1
            "#,
            id
        )
        .fetch_optional(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let parent = record.map(|r| r.parent_id);

        let records = sqlx::query!(
            r#"
                select child_id
                from RequirementHierarchies
                where parent_id = $1
                order by child_id
            "#,
            id
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let children = records.into_iter().map(|r| r.child_id).collect();
        let leaf_statistic = LeafChildrenStatistic::try_from(db, &id).await?;

        let trace_info = RequirementTraceInfo::try_from(db, &id).await?;
        let test_coverage_info = RequirementTestCoverageInfo::try_from(db, &id).await?;

        let records = sqlx::query!(
            r#"
                select review_name, review_date, comment
                from ManuallyVerified
                where req_id = $1
                order by review_name, review_date
            "#,
            id
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let mut verified_info = Vec::with_capacity(records.len());
        for record in records {
            verified_info.push(VerifiedRequirementInfo {
                review_name: record.review_name,
                review_date: PrimitiveDateTime::parse(
                    &record.review_date,
                    super::REVIEW_DATE_FORMAT,
                )
                .expect("Review date was added to db in custom review-date format."),
                comment: record.comment,
            });
        }

        let valid = sqlx::query!(
            r#"
                select * from InvalidRequirements
                where id = $1
            "#,
            id
        )
        .fetch_optional(db.pool())
        .await
        .map_err(ReportError::Db)?
        .is_none();

        let rendered_info = if let Some(template) = info_template {
            let template_content = tokio::fs::read_to_string(template)
                .await
                .map_err(|_| ReportError::Template)?;

            if let Some(value) = &info {
                let context = tera::Context::from_serialize(value)
                    .expect("Requirement info value is valid JSON.");
                let rendered = tera::Tera::one_off(&template_content, &context, true)
                    .map_err(|_| ReportError::Tera)?;
                Some(rendered)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            meta: Requirement {
                id,
                title,
                link,
                manual,
                deprecated,
                info,
            },
            parent,
            rendered_info,
            direct_children: children,
            leaf_statistic,
            trace_info,
            test_coverage_info,
            verified_info,
            valid,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct LeafChildrenStatistic {
    leaf_cnt: i32,
    traced_leaf_cnt: i32,
    traced_leaf_ratio: f64,
    covered_leaf_cnt: i32,
    covered_leaf_ratio: f64,
    passed_covered_leaf_cnt: i32,
    passed_covered_leaf_ratio: f64,
}

impl LeafChildrenStatistic {
    pub async fn try_from(db: &MantraDb, id: &str) -> Result<Option<Self>, ReportError> {
        sqlx::query_as!(
            LeafChildrenStatistic,
            r#"
                select 
                leaf_cnt as "leaf_cnt!: i32",
                traced_leaf_cnt as "traced_leaf_cnt!: i32",
                traced_leaf_ratio as "traced_leaf_ratio!: f64",
                covered_leaf_cnt as "covered_leaf_cnt!: i32",
                covered_leaf_ratio as "covered_leaf_ratio!: f64",
                passed_covered_leaf_cnt as "passed_covered_leaf_cnt!: i32",
                passed_covered_leaf_ratio as "passed_covered_leaf_ratio!: f64"
                from LeafChildOverview
                where id = $1
            "#,
            id
        )
        .fetch_optional(db.pool())
        .await
        .map_err(ReportError::Db)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementTraceInfo {
    pub traced: bool,
    pub fully_traced: bool,
    pub direct_traces: Vec<TraceLocation>,
    pub indirect_traces: Vec<IndirectTraceInfo>,
}

impl RequirementTraceInfo {
    pub async fn try_from(db: &MantraDb, id: &str) -> Result<Self, ReportError> {
        let records = sqlx::query_as!(
            TraceLocation,
            r#"
            select filepath, line as "line: Line"
            from Traces
            where req_id = $1
            order by filepath, line
        "#,
            id
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let direct_traces = records;

        let records = sqlx::query!(
            r#"
            select traced_id as "traced_id!", traces as "traces!: String"
            from IndirectTraceTree
            where id = $1
            order by traced_id
        "#,
            id
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let mut indirect_traces = Vec::with_capacity(records.len());

        for record in records {
            indirect_traces.push(IndirectTraceInfo {
                traced_id: record.traced_id,
                traces: serde_json::from_str(&record.traces)
                    .expect("Traces extracted as JSON from DB."),
            })
        }

        let fully_traced = sqlx::query!(
            r#"
                select *
                from FullyTracedRequirements
                where id = $1
            "#,
            id
        )
        .fetch_optional(db.pool())
        .await
        .map_err(ReportError::Db)?
        .is_some();

        Ok(Self {
            traced: !direct_traces.is_empty() || !indirect_traces.is_empty(),
            fully_traced,
            direct_traces,
            indirect_traces,
        })
    }
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, sqlx::Type, schemars::JsonSchema,
)]
pub struct TraceLocation {
    pub filepath: String,
    pub line: Line,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct IndirectTraceInfo {
    pub traced_id: String,
    pub traces: Vec<TraceLocation>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementTestCoverageInfo {
    pub covered: bool,
    pub passed: bool,
    pub fully_covered: bool,
    pub direct_coverage: Vec<TestCoverageTestRunInfo>,
    pub indirect_coverage: Vec<IndirectTestCoverageInfo>,
}

impl RequirementTestCoverageInfo {
    pub async fn try_from(db: &MantraDb, id: &str) -> Result<Self, ReportError> {
        let records = sqlx::query!(
            r#"
                select test_run_name, test_run_date, tests as "tests!: String"
                from DirectCoverageTree
                where id = $1
                order by test_run_name, test_run_date
            "#,
            id
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let mut direct_coverage = Vec::with_capacity(records.len());

        for record in records {
            direct_coverage.push(TestCoverageTestRunInfo {
                name: record.test_run_name,
                date: iso8601_str_to_offsetdatetime(&record.test_run_date),
                tests: serde_json::from_str(&record.tests)
                    .expect("Tests extracted as JSON from DB."),
            })
        }

        let records = sqlx::query!(
            r#"
                select covered_id, test_runs as "test_runs!: String"
                from IndirectTestCoverageTree
                where id = $1
                order by covered_id
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
                test_runs: serde_json::from_str(&record.test_runs)
                    .expect("Test runs extracted as JSON from DB."),
            })
        }

        let passed = sqlx::query!(
            "
            select *
            from PassedCoveredRequirements
            where id = $1
            ",
            id
        )
        .fetch_one(db.pool())
        .await
        .ok()
        .is_some();

        let fully_covered = sqlx::query!(
            r#"
                select *
                from FullyCoveredRequirements
                where id = $1
            "#,
            id
        )
        .fetch_optional(db.pool())
        .await
        .map_err(ReportError::Db)?
        .is_some();

        Ok(Self {
            covered: !direct_coverage.is_empty() || !indirect_coverage.is_empty(),
            passed,
            fully_covered,
            direct_coverage,
            indirect_coverage,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TestCoverageTestRunInfo {
    pub name: String,
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    #[schemars(with = "String")]
    pub date: OffsetDateTime,
    pub tests: Vec<TestCoverageTestInfo>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TestCoverageTestInfo {
    pub name: String,
    pub passed: bool,
    pub traces: Vec<TraceLocation>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct IndirectTestCoverageInfo {
    pub covered_id: String,
    pub test_runs: Vec<TestCoverageTestRunInfo>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct VerifiedRequirementInfo {
    pub review_name: String,
    #[serde(with = "super::review_date_format")]
    #[schemars(with = "String")]
    pub review_date: PrimitiveDateTime,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TestStatistics {
    pub overview: TestsOverview,
    pub test_runs: Vec<TestRunInfo>,
}

impl TestStatistics {
    pub async fn try_from(
        db: &MantraDb,
        test_run_template: Option<&Path>,
    ) -> Result<Self, ReportError> {
        let overview = TestsOverview::try_from(db).await?;

        let test_run_records = sqlx::query!(
            "
            select name, date
            from TestRuns
            order by name, date
            "
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let mut test_runs = Vec::new();

        for test_run in test_run_records {
            let date = iso8601_str_to_offsetdatetime(&test_run.date);
            test_runs
                .push(TestRunInfo::try_from(db, test_run.name, date, test_run_template).await?);
        }

        Ok(Self {
            overview,
            test_runs,
        })
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TestRunInfo {
    pub overview: TestRunOverview,
    pub name: String,
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    #[schemars(with = "String")]
    pub date: OffsetDateTime,
    pub meta: Option<serde_json::Value>,
    pub rendered_meta: Option<String>,
    pub logs: Option<String>,
    pub tests: Vec<TestInfo>,
}

impl TestRunInfo {
    pub async fn try_from(
        db: &MantraDb,
        name: impl Into<String>,
        date: OffsetDateTime,
        test_run_template: Option<&Path>,
    ) -> Result<Self, ReportError> {
        let name: String = name.into();
        let overview = TestRunOverview::try_from(db, &name, &date).await?;

        let test_records = sqlx::query!(
            r#"
            select name, passed as "passed!: bool", skipped as "skipped!: bool", reason as "reason?: String", filepath, line as "line: Line" from (
                select
                name, filepath, line,
                passed,
                false as skipped,
                null as reason
                from Tests
                where test_run_name = $1 and test_run_date = $2
                
                union all
                
                select
                name, filepath, line,
                false as passed,
                true as skipped,
                reason
                from SkippedTests
                where test_run_name = $1 and test_run_date = $2
            )
            order by name, filepath, line
        "#,
            name,
            date
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let mut test_info = Vec::new();

        for test in test_records {
            let covers = sqlx::query!(
                "
                select req_id from TestCoverage
                where test_run_name = $1 and
                test_run_date = $2 and
                test_name = $3
                order by req_id
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

            test_info.push(TestInfo {
                covers,
                name: test.name,
                filepath: PathBuf::from(test.filepath),
                line: test.line,
                state,
            })
        }

        let record = sqlx::query!(
            r#"
            select meta, logs from TestRuns
            where name = $1 and date = $2
            "#,
            name,
            date
        )
        .fetch_one(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let meta = record
            .meta
            .map(|m| serde_json::from_str(&m).expect("Test run meta data must be valid JSON."));

        let rendered_meta = if let Some(template) = test_run_template {
            let template_content = tokio::fs::read_to_string(template)
                .await
                .map_err(|_| ReportError::Template)?;

            if let Some(value) = &meta {
                let context = tera::Context::from_serialize(value)
                    .expect("Test-run meta value is valid JSON.");
                let rendered = tera::Tera::one_off(&template_content, &context, true)
                    .map_err(|_| ReportError::Tera)?;
                Some(rendered)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            overview,
            name,
            date,
            meta,
            rendered_meta,
            logs: record.logs,
            tests: test_info,
        })
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TestInfo {
    /// List of requirements that are covered by this test.
    pub covers: Vec<String>,
    pub name: String,
    pub filepath: PathBuf,
    pub line: Line,
    pub state: TestState,
}

impl Review {
    pub async fn try_from(
        db: &MantraDb,
        name: impl Into<String>,
        date: PrimitiveDateTime,
    ) -> Result<Self, ReportError> {
        let name: String = name.into();

        let requirements = sqlx::query_as!(
            VerifiedRequirement,
            "
                select req_id as id, comment
                from ManuallyVerified
                where review_name = $1 and review_date = $2
                order by req_id
            ",
            name,
            date
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

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
            requirements,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Unrelated {
    pub traces: Vec<TracePk>,
    pub coverage: Vec<UnrelatedCoverage>,
    pub verified_requirements: Vec<UnrelatedVerified>,
}

impl Unrelated {
    pub async fn try_from(db: &MantraDb) -> Result<Self, ReportError> {
        let traces = sqlx::query_as!(
            TracePk,
            r#"
                select
                req_id,
                filepath,
                line as "line!: u32"
                from UnrelatedTraces
            "#
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let coverage = UnrelatedCoverage::try_from(db).await?;
        let verified = UnrelatedVerified::try_from(db).await?;

        Ok(Self {
            traces,
            coverage,
            verified_requirements: verified,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct UnrelatedCoverage {
    pub test_run_name: String,
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    #[schemars(with = "String")]
    pub test_run_date: OffsetDateTime,
    pub test_name: String,
    pub req_id: String,
    pub trace_filepath: PathBuf,
    pub trace_line: Line,
}

impl UnrelatedCoverage {
    pub async fn try_from(db: &MantraDb) -> Result<Vec<Self>, ReportError> {
        let records = sqlx::query!(
            r#"
                select
                test_run_name,
                test_run_date,
                test_name,
                req_id,
                trace_filepath,
                trace_line as "trace_line!: u32"
                from UnrelatedTestCoverage
            "#,
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let mut unrelated = Vec::new();

        for record in records {
            unrelated.push(UnrelatedCoverage {
                test_run_name: record.test_run_name,
                test_run_date: iso8601_str_to_offsetdatetime(&record.test_run_date),
                test_name: record.test_name,
                req_id: record.req_id,
                trace_filepath: PathBuf::from(record.trace_filepath),
                trace_line: record.trace_line,
            })
        }

        Ok(unrelated)
    }
}

time::serde::format_description!(
    review_date_format,
    PrimitiveDateTime,
    mantra_schema::reviews::REVIEW_DATE_FORMAT
);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct UnrelatedVerified {
    pub review_name: String,
    #[serde(with = "review_date_format")]
    #[schemars(with = "String")]
    pub review_date: PrimitiveDateTime,
    pub req_id: String,
    pub comment: Option<String>,
}

impl UnrelatedVerified {
    pub async fn try_from(db: &MantraDb) -> Result<Vec<Self>, ReportError> {
        let records = sqlx::query!(
            r#"
                select * from UnrelatedManuallyVerified
            "#,
        )
        .fetch_all(db.pool())
        .await
        .map_err(ReportError::Db)?;

        let mut unrelated = Vec::new();

        for record in records {
            unrelated.push(UnrelatedVerified {
                review_name: record.review_name,
                review_date: mantra_schema::reviews::date_from_str(&record.review_date)
                    .expect("Review date was serialized before."),
                req_id: record.req_id,
                comment: record.comment,
            });
        }

        Ok(unrelated)
    }
}
