use jiff::Zoned;
use relative_path::RelativePathBuf;

use crate::{
    ConversionError, Line, Origin, Properties, REVIEWS_FOLDER_NAME,
    encoding::TargetEncoding,
    product::ProductId,
    report::{
        ResolvedRevision, formattable_content::FormattableContent, product::ProductMetadata,
        requirement::RequirementReference, test_case::TestCaseReference,
        test_run::TestRunReference,
    },
    requirements::ReqId,
    test_runs::TestCaseState,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ReviewReportSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    pub product: ProductMetadata,
    /// The name of the review.
    /// [req("review.id")]
    pub name: String,
    /// The UTC date and time the review was started.
    ///
    /// **Note:** The date must be given adhering to the Temporal ISO 8601 grammar.
    /// See: https://tc39.es/proposal-temporal/#sec-temporal-iso8601grammar
    ///
    /// [req("review.id")]
    #[schemars(with = "String")]
    pub utc_date: Zoned,
    pub state: ReviewState,
    /// The authors that were part of the review.
    /// [req("review.authors")]
    pub authors: Vec<String>,
    /// Optional description of the review.
    /// [req("review.description")]
    pub description: Option<FormattableContent>,
    /// Optional origin of the review.
    /// [req("review.origin")]
    pub origin: Option<Origin>,
    pub schema_origin: Option<Origin>,
    pub config_origin: Option<Origin>,
    /// Optional properties related to this review.
    pub properties: Option<Properties>,
    pub schema_properties: Option<Properties>,
    pub config_properties: Option<Properties>,
    /// Optional revisions for the review.
    pub revisions: Option<Vec<ResolvedRevision>>,
    /// List of requirements that are verified in this review.
    /// [req("review.verify_req")]
    pub requirements: Option<Vec<VerifiedRequirement>>,
    /// List of test run overrides added with this review.
    /// [req("review.test_case_state", "review.coverage")]
    pub test_run_overrides: Option<Vec<ResolvedOverrideTestRun>>,
    pub ignored_entries: Option<IgnoredEntries>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct VerifiedRequirement {
    pub req: RequirementReference,
    pub rational: FormattableContent,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct IgnoredEntries {
    pub total: i64,
    pub requirements: Option<Vec<IgnoredManualRequirementVerification>>,
    pub test_case_state_overrides: Option<Vec<IgnoredTestCaseStateOverride>>,
    pub test_case_line_coverage_overrides: Option<Vec<IgnoredTestCaseLineCoverageOverride>>,
    pub test_run_line_coverage_overrides: Option<Vec<IgnoredTestRunLineCoverageOverride>>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct IgnoredManualRequirementVerification {
    pub id: ReqId,
    pub rational: String,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct IgnoredTestCaseStateOverride {
    pub test_run_name: String,
    pub test_case_name: String,
    pub state: TestCaseState,
    pub rational: FormattableContent,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct IgnoredTestCaseLineCoverageOverride {
    pub test_run_name: String,
    pub test_case_name: String,
    #[schemars(with = "String")]
    pub cov_filepath: RelativePathBuf,
    pub cov_line: Line,
    pub covered: Option<bool>,
    pub rational: FormattableContent,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct IgnoredTestRunLineCoverageOverride {
    pub test_run_name: String,
    #[schemars(with = "String")]
    pub cov_filepath: RelativePathBuf,
    pub cov_line: Line,
    pub covered: Option<bool>,
    pub rational: FormattableContent,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ReviewReference {
    pub product_id: ProductId,
    pub name: String,
    pub state: ReviewState,
}

impl ReviewReference {
    pub fn url_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Url)
    }

    pub fn os_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Os)
    }

    fn encode_path(&self, target: TargetEncoding) -> RelativePathBuf {
        let product_path = match target {
            TargetEncoding::Os => self.product_id.os_path(),
            TargetEncoding::Url => self.product_id.url_path(),
        };

        let limit_name = crate::encoding::limit_str_len(&self.name);
        let encoded_ref = crate::encoding::encode(&limit_name, target);

        product_path.join(REVIEWS_FOLDER_NAME).join(encoded_ref)
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    Obsolete = 0,
    Valid = 1,
}

impl ReviewState {
    pub fn as_number(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<i64> for ReviewState {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ReviewState::Obsolete),
            1 => Ok(ReviewState::Valid),
            _ => Err(ConversionError::UnknownState),
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedOverrideTestRun {
    pub test_run: TestRunReference,
    pub test_cases: Option<Vec<ResolvedOverrideTestCase>>,
    pub coverage: Option<Vec<ResolvedOverrideFileCoverage>>,
}

/// Represents a test case state override in a review.
/// [req("review.test_case_state", "review.coverage")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ResolvedOverrideTestCase {
    /// Name of the test case whose state and/or related code coverage is overridden in a review.
    pub test_case: TestCaseReference,
    pub state: Option<ResolvedOverrideTestCaseState>,
    pub coverage: Option<Vec<ResolvedOverrideFileCoverage>>,
}

/// Represents a test case state override in a review.
/// [req("review.test_case_state")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ResolvedOverrideTestCaseState {
    pub old: TestCaseState,
    pub new: TestCaseState,
    pub rational: FormattableContent,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ResolvedOverrideFileCoverage {
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    pub lines: Vec<ResolvedOverrideCoveredLineInfo>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ResolvedOverrideCoveredLineInfo {
    pub nr: Line,
    pub original_hits: Option<i64>,
    pub covered: Option<bool>,
    pub rational: FormattableContent,
}
