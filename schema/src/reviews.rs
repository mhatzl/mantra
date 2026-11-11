use std::ops::Deref;

use jiff::Zoned;
use relative_path::RelativePathBuf;

use crate::{
    Line, Origin, Properties, Revision, media_type::MediaType, product::ProductId,
    test_runs::TestCaseState,
};

use super::requirements::ReqId;

/// Defines the schema to exchange review related information.
/// [req("exchange.review.schema")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReviewSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    /// Optional product ID to specify the product the reviews are part of.
    /// If this field is not set, the ID of the product whose configuration included this schema is used.
    pub product_id: Option<ProductId>,
    pub reviews: Vec<Review>,
    /// Optional properties related to all reviews in this entry.
    ///
    /// **Note:** If a review sets a property key directly,
    /// the value set at the review will be taken.
    pub properties: Option<Properties>,
    /// Optional base origin of the reviews in this entry.
    /// e.g. specific branch or commit from a git repository
    pub origin: Option<Origin>,
    /// Optional MIME/Media type of the general text added with this schema.
    /// e.g rationals and revision summary
    pub media_type: Option<MediaType>,
}

impl crate::Schema for ReviewSchema {
    fn version(&self) -> &str {
        if let Some(version) = &self.schema_version {
            version
        } else {
            crate::SCHEMA_VERSION
        }
    }

    fn product_id(&self) -> &Option<ProductId> {
        &self.product_id
    }

    fn origin(&self) -> &Option<Origin> {
        &self.origin
    }

    fn media_type(&self) -> &Option<MediaType> {
        &self.media_type
    }
}

/// Defines the fields for a review.
/// [req("exchange.review")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Review {
    /// The name of the review.
    /// [req("review.id")]
    pub name: String,
    /// The UTC date and time the review was started.
    ///
    /// **Note:** The date must be given adhering to the Temporal ISO 8601 grammar.
    /// See: https://tc39.es/proposal-temporal/#sec-temporal-iso8601grammar
    ///
    /// If no time zone is set, the system one is used as fallback.
    ///
    /// [req("review.id")]
    #[serde(deserialize_with = "crate::deserialize_zoned")]
    #[schemars(with = "String")]
    pub utc_date: Zoned,
    /// The authors that were part of the review.
    /// [req("review.authors")]
    pub authors: Vec<String>,
    /// Optional description of the review.
    /// [req("review.description")]
    pub description: Option<String>,
    /// Optional origin of the review.
    /// [req("review.origin")]
    pub origin: Option<Origin>,
    /// Optional properties related to this review.
    pub properties: Option<Properties>,
    /// Optional revisions for the review.
    pub revisions: Option<Vec<Revision>>,
    /// List of requirements that are verified in this review.
    /// [req("review.verify_req")]
    #[serde(alias = "requirement", default)]
    pub requirements: Vec<VerifiedRequirementGroup>,
    /// List of test run overrides added with this review.
    /// [req("review.test_case_state", "review.coverage")]
    #[serde(alias = "override", default)]
    pub test_run_overrides: Vec<OverrideTestRun>,
}

/// Represents a verification entry affecting one or more requirements.
/// [req("review.verify_req")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct VerifiedRequirementGroup {
    /// One or more requirement IDs to mark the related requirements as manually verified by a review.
    #[serde(alias = "ids")]
    pub id: OneOrMultRequirementIds,
    /// Mandatory comment explaining the manual verification.
    pub rational: String,
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

impl std::fmt::Display for OneOrMultRequirementIds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            OneOrMultRequirementIds::One(req_id) => write!(f, "{}", req_id),
            OneOrMultRequirementIds::Mult(req_ids) => {
                write!(
                    f,
                    "{}",
                    req_ids
                        .iter()
                        .map(|r| r.deref().clone())
                        .collect::<Vec<String>>()
                        .join(",")
                )
            }
        }
    }
}

/// Represents review overrides for a specific test run.
/// [req("review.test_case_state", "review.coverage")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct OverrideTestRun {
    /// Name of the test run.
    pub name: String,
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct OverrideTestCaseState {
    /// The new state that is set with this override.
    pub new: TestCaseState,
    /// Mandatory comment explaining why the state is overriden via review.
    pub rational: String,
}

/// Code coverage overrides per file.
/// [req("review.coverage")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct OverrideFileCoverage {
    /// The file whose coverage data is overridden in a review.
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    /// The line information in the file that is overridden.
    #[serde(default)]
    pub lines: Vec<OverrideCoveredLineInfo>,
}

/// Code coverage override of one or more lines.
/// [req("review.coverage")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct OverrideCoveredLineInfo {
    /// The number of lines affected by this override.
    #[serde(alias = "number")]
    pub numbers: Vec<Line>,
    /// Override if the lines were covered during a test run or test case execution.
    /// If None, the line is marked to be excluded from line coverage analysis.
    pub covered: Option<bool>,
    /// Mandatory comment explaining the change to the code coverage information.
    pub rational: String,
}
