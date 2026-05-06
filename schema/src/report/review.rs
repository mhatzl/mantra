use crate::{
    ConversionError, Origin, Properties, Revision, product::ProductId,
    report::product::ProductMetadata, requirements::ReqId, reviews::OverrideTestRun,
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
    /// [req("review.id")]
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    /// The authors that were part of the review.
    /// [req("review.authors")]
    pub authors: Vec<String>,
    /// Optional description of the review.
    /// [req("review.description")]
    pub description: Option<String>,
    /// Optional origin of the review.
    /// [req("review.origin")]
    pub origin: Option<Origin>,
    pub base_origin: Option<Origin>,
    /// Optional properties related to this review.
    pub properties: Option<Properties>,
    pub base_properties: Option<Properties>,
    /// Optional revisions for the review.
    pub revisions: Option<Vec<Revision>>,
    /// List of requirements that are verified in this review.
    /// [req("review.verify_req")]
    pub requirements: Vec<VerifiedRequirement>,
    /// List of test run overrides added with this review.
    /// [req("review.test_case_state", "review.coverage")]
    pub test_run_overrides: Vec<OverrideTestRun>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct VerifiedRequirement {
    pub id: ReqId,
    pub comment: String,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ReviewReference {
    pub product_id: ProductId,
    pub name: String,
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    pub state: ReviewState,
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
    pub fn as_nr(&self) -> i32 {
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
