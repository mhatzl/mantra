use std::path::PathBuf;

use time::OffsetDateTime;

use crate::{
    testcov::{TestCaseState, TestRunId},
    Line,
};

use super::requirements::ReqId;

/// A simplified format to specify the UTC date and time of a review.
///
/// **Examples:**
/// - `2025-04-26 10:30utc+01`
/// - `2025-10-20T12:30:10.147utc-04`
pub const REVIEW_DATE_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day][first [T] [ ]][hour]:[minute][optional [:[second][optional [.[subsecond]]]]]utc[offset_hour sign:mandatory]"
);

time::serde::format_description!(review_date_format, OffsetDateTime, REVIEW_DATE_FORMAT);

/// Tries to convert the given string to an [`OffsetDateTime`] using the [`REVIEW_DATE_FORMAT`].
pub fn date_from_str(date: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(date, REVIEW_DATE_FORMAT)
}

/// Defines the schema to exchange review related information.
/// [req("exchange.review.schema")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct ReviewSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    pub reviews: Vec<Review>,
    /// Optional metadata related to all reviews in this entry.
    pub metadata: Option<serde_json::Value>,
    /// Optional base origin of the reviews in this entry.
    /// e.g. specific branch or commit from a git repository
    pub origin: Option<serde_json::Value>,
}

/// Defines the fields for a review.
/// [req("exchange.review")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct Review {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    /// The name of the review.
    /// [req("review.id")]
    pub name: String,
    /// The UTC date and time the review was started.
    /// [req("review.id")]
    #[serde(with = "review_date_format")]
    #[schemars(
        with = "String",
        regex(
            pattern = r"(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})(T| )(?<hour>\d{2}):(?<minute>\d{2})(?<second>:\d{2}(?<subsecond>\.\d{3})?)?utc(?<utc>(-|+)\d{2})"
        )
    )]
    pub date: OffsetDateTime,
    /// Hash of the review content to detect changes.
    ///
    /// If not provided, will be computed using the fields: reviewer, description, origin, requirements, overrides
    /// [req("changes.track.reviews")]
    pub content_hash: Option<String>,
    /// The reviewer that were part of the review.
    /// [req("review.reviewer")]
    pub reviewer: String,
    /// Optional description of the review.
    /// [req("review.description")]
    pub description: Option<String>,
    /// Optional origin of the review.
    /// [req("review.origin")]
    pub origin: Option<serde_json::Value>,
    /// List of requirements that are verified in this review.
    /// [req("review.verify_req")]
    #[serde(alias = "requirement", default)]
    pub requirements: Vec<VerifiedRequirement>,
    /// List of test run overrides added with this review.
    /// [req("review.test_case_state", "review.coverage")]
    #[serde(alias = "override", default)]
    pub overrides: Vec<OverrideTestRun>,
}

/// Represents a verification entry affecting one or more requirements.
/// [req("review.verify_req")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct VerifiedRequirement {
    /// One or more requirement IDs to mark the related requirements as manually verified by a review.
    #[serde(alias = "ids")]
    pub id: OneOrMultRequirementIds,
    /// Mandatory comment explaining the manual verification.
    pub comment: String,
}

/// Variant to allow setting either one or more requirement IDs for manual verification.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(untagged)]
pub enum OneOrMultRequirementIds {
    /// Only one requirement is verified by a verification entry.
    One(ReqId),
    /// List of requirements that are all verified by one verification entry.
    Mult(Vec<ReqId>),
}

/// Represents review overrides for a specific test run.
/// [req("review.test_case_state", "review.coverage")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct OverrideTestRun {
    /// Identification of the test run the overrides are applied to.
    pub test_run: TestRunId,
    /// List of test case state overrides.
    /// [req("review.test_case_state")]
    #[serde(alias = "test", default)]
    pub test_cases: Vec<OverrideTestCase>,
    /// List of file coverage overrides.
    /// [req("review.coverage")]
    #[serde(default)]
    pub coverage: Vec<OverrideFileCoverage>,
}

/// Represents a test case state override in a review.
/// [req("review.test_case_state", "review.coverage")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct OverrideTestCase {
    /// Name of the test case whose state and/or related code coverage is overridden in a review.
    pub name: String,
    /// Overrides the state of the test case.
    /// [req("review.test_case_state")]
    pub state: Option<OverrideTestCaseState>,
    /// Overrides the code coverage related to this test case.
    /// [req("review.coverage")]
    #[serde(default)]
    pub coverage: Vec<OverrideFileCoverage>,
}

/// Represents a test case state override in a review.
/// [req("review.test_case_state")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct OverrideTestCaseState {
    /// The new state that is set with this override.
    pub new: TestCaseState,
    /// Mandatory comment explaining why the state is overriden via review.
    pub comment: String,
}

/// Code coverage overrides per file.
/// [req("review.coverage")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct OverrideFileCoverage {
    /// The file whose coverage data is overridden in a review.
    pub filepath: PathBuf,
    /// The line information in the file that is overridden.
    #[serde(default)]
    pub lines: Vec<OverrideCoveredLineInfo>,
}

/// Code coverage override of one or more lines.
/// [req("review.coverage")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct OverrideCoveredLineInfo {
    /// The number of lines affected by this override.
    #[serde(alias = "nr")]
    pub nrs: Vec<Line>,
    /// The new number of times the set lines are reached during a test run or test case execution.
    pub hits: usize,
    /// Mandatory comment explaining the change to the code coverage information.
    pub comment: String,
}
