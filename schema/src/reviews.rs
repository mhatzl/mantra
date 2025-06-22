use std::path::PathBuf;

use time::PrimitiveDateTime;

use crate::{
    testcov::{TestCaseState, TestRunId},
    Line,
};

use super::requirements::ReqId;

pub const REVIEW_DATE_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day] [hour]:[minute][optional [:[second][optional [.[subsecond]]]]]"
);

time::serde::format_description!(review_date_format, PrimitiveDateTime, REVIEW_DATE_FORMAT);

pub fn date_from_str(date: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
    PrimitiveDateTime::parse(date, REVIEW_DATE_FORMAT)
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct ReviewSchema {
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    pub name: String,
    #[serde(with = "review_date_format")]
    #[schemars(
        with = "String",
        regex(
            pattern = r"(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2}) (?<hour>\d{2}):(?<minute>\d{2})(?<second>:\d{2}(?<subsecond>\.\d{3})?)?"
        )
    )]
    pub date: PrimitiveDateTime,
    /// Hash of the review content to detect changes.
    ///
    /// If not provided, will be computed using the fields: reviewer, comment, requirements, overrides
    pub content_hash: Option<String>,
    pub reviewer: String,
    pub comment: Option<String>,
    #[serde(alias = "requirement")]
    pub requirements: Vec<VerifiedRequirement>,
    #[serde(alias = "override")]
    pub overrides: Vec<OverrideTestRun>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct VerifiedRequirement {
    #[serde(alias = "ids")]
    pub id: OneOrMultRequirementIds,
    pub comment: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub enum OneOrMultRequirementIds {
    One(ReqId),
    Mult(Vec<ReqId>),
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct OverrideTestRun {
    pub test_run: TestRunId,
    #[serde(alias = "test")]
    pub test_cases: Vec<OverrideTestCaseState>,
    pub coverage: Vec<OverrideFileCoverage>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct OverrideTestCaseState {
    pub name: String,
    pub state: TestCaseState,
    pub comment: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct OverrideFileCoverage {
    pub filepath: PathBuf,
    #[serde(default)]
    pub statements: Vec<OverrideStatementCoverage>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct OverrideStatementCoverage {
    pub lines: Vec<Line>,
    pub hits: usize,
    pub comment: Option<String>,
}
