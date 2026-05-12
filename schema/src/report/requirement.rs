use relative_path::RelativePathBuf;

use crate::{
    ConversionError, Origin, Properties, REQUIREMENTS_FOLDER_NAME,
    encoding::TargetEncoding,
    product::ProductId,
    report::{
        annotations::{TraceReference, TracesSummary},
        product::ProductMetadata,
        review::ReviewState,
        tests::TestState,
    },
    requirements::ReqId,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RequirementReportSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    pub state: RequirementState,
    pub parents: Option<Vec<RequirementReference>>,
    pub children: Option<Vec<RequirementReference>>,
    pub traces: Option<RequirementTracesOverview>,
    pub covered_by: Option<RequirementCoverageByTests>,
    pub reviewed_in: Option<Vec<RequirementReviewReference>>,
    pub product: ProductMetadata,
    /// ID of the requirement.
    /// [req("req.id")]
    pub id: ReqId,
    /// Title of the requirement.
    /// [req("req.title")]
    pub title: String,
    /// Optional description of the requirement.
    /// [req("req.description")]
    pub description: Option<String>,
    pub base_origin: Option<Origin>,
    /// Origin where the requirement is defined at.
    /// [req("req.origin")]
    pub origin: Option<Origin>,
    /// true: Marks the requirement to require manual verification.
    ///
    /// **Note:** All potential children of such a requirement are also marked
    /// to require manual verification.
    /// [req("req.manual")]
    pub manual_verification: bool,
    /// true: Marks the requirement to be deprecated.
    ///
    /// **Note:** All potential children of such a requirement are also marked as deprecated.
    /// [req("req.deprecated")]
    pub deprecated: bool,
    /// true: Instructs mantra to ignore the requirement for the product it is mapped to.
    ///
    /// **Note:** All potential children of such a requirement will also be ignored.
    /// [req("req.ignore")]
    pub ignored: bool,
    /// true: Instructs mantra to treat the requirement for the product as optional.
    ///
    /// **Note:** All potential children of such a requirement are also marked as optional.
    /// [req("req.ignore")]
    pub optional: bool,
    /// List of custom properties of a requirement.
    /// [req("req.properties")]
    pub properties: Option<Properties>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct RequirementReference {
    pub product_id: ProductId,
    pub id: ReqId,
    pub state: RequirementState,
    pub optional: bool,
}

impl RequirementReference {
    pub fn url_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Url)
    }

    pub fn os_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Os)
    }

    fn encode_path(&self, target: TargetEncoding) -> RelativePathBuf {
        let req_path = if self.id.contains('.') {
            RelativePathBuf::from_iter(
                self.id
                    .split('.')
                    .map(|id| crate::encoding::encode(&id, target).to_string()),
            )
        } else {
            RelativePathBuf::from(crate::encoding::encode(&self.id, target).to_string())
        };

        let product_path = match target {
            TargetEncoding::Os => self.product_id.os_path(),
            TargetEncoding::Url => self.product_id.url_path(),
        };

        product_path.join(REQUIREMENTS_FOLDER_NAME).join(req_path)
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
pub enum RequirementState {
    Failed = 0,
    Verified = 1,
    Skipped = 2,
    Unverified = 3,
    Deprecated = 4,
    Ignored = 5,
}

impl RequirementState {
    pub fn as_nr(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<i64> for RequirementState {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(RequirementState::Failed),
            1 => Ok(RequirementState::Verified),
            2 => Ok(RequirementState::Skipped),
            3 => Ok(RequirementState::Unverified),
            4 => Ok(RequirementState::Deprecated),
            5 => Ok(RequirementState::Ignored),
            _ => Err(ConversionError::UnknownState),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementTracesOverview {
    pub summary: TracesSummary,
    pub all: Vec<TraceReference>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RequirementCoverageByTests {
    pub test_runs: Vec<RequirementCoverageByTestRuns>,
    pub test_cases: Vec<RequirementCoverageByTestCases>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RequirementCoverageByTestRuns {
    pub product_id: ProductId,
    pub name: String,
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    pub state: TestState,
    pub covered_traces: Option<Vec<TraceReference>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RequirementCoverageByTestCases {
    pub product_id: ProductId,
    pub test_run_name: String,
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub test_run_date: time::OffsetDateTime,
    pub test_case_name: String,
    pub state: TestState,
    pub covered_traces: Option<Vec<TraceReference>>,
    pub directly_verified: bool,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct RequirementReviewReference {
    pub product_id: ProductId,
    pub name: String,
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    pub state: ReviewState,
    pub comment: String,
}
