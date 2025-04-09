use std::path::PathBuf;

use time::PrimitiveDateTime;

use crate::Line;

use super::requirements::ReqId;

pub const REVIEW_DATE_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day] [hour]:[minute][optional [:[second][optional [.[subsecond]]]]]"
);

time::serde::format_description!(review_date_format, PrimitiveDateTime, REVIEW_DATE_FORMAT);

pub fn date_from_str(date: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
    PrimitiveDateTime::parse(date, REVIEW_DATE_FORMAT)
}

#[derive(
    Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
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
    #[serde(alias = "content-hash")]
    pub content_hash: Option<String>,
    pub reviewer: String,
    pub comment: Option<String>,
    #[serde(alias = "requirement")]
    pub requirements: Vec<VerifiedRequirement>,
    #[serde(alias = "override")]
    pub overrides: Vec<TestCovOverrides>,
}

#[derive(
    Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct VerifiedRequirement {
    pub id: ReqId,
    pub comment: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TestCovOverrides {
    pub test_run_name: String,
    /// Test run date must be given in ISO8601 format.
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    #[schemars(with = "String")]
    pub test_run_date: time::OffsetDateTime,
    pub tests: Vec<TestOverride>,
    pub statement_coverage: Vec<OverrideCoveredFile>,
}

#[derive(
    Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TestOverride {
    pub name: String,
    pub state: OverrideTestState,
    pub comment: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum OverrideTestState {
    Passed,
    Failed,
    Skipped,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct OverrideCoveredFile {
    pub filepath: PathBuf,
    #[serde(default)]
    pub statements: Vec<OverrideCoveredStatement>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct OverrideCoveredStatement {
    pub lines: Vec<Line>,
    pub hits: usize,
    pub comment: Option<String>,
}
