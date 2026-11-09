use relative_path::RelativePathBuf;

use crate::{
    ConversionError, Origin, Properties, REQUIREMENTS_FOLDER_NAME,
    encoding::TargetEncoding,
    product::ProductId,
    report::{
        annotations::{TraceReference, TracesSummary},
        formattable_content::FormattableContent,
        product::ProductMetadata,
        review::ReviewReference,
        tests::TestReference,
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
    pub covered_by: Option<Vec<RequirementCoverage>>,
    pub reviewed_in: Option<Vec<RequirementReview>>,
    pub product: ProductMetadata,
    /// ID of the requirement.
    /// [req("req.id")]
    pub id: ReqId,
    /// Title of the requirement.
    /// [req("req.title")]
    pub title: FormattableContent,
    /// Optional description of the requirement.
    /// [req("req.description")]
    pub description: Option<FormattableContent>,
    pub kind: Option<String>,
    /// Origin where the requirement is defined at.
    /// [req("req.origin")]
    pub origin: Option<Origin>,
    pub schema_origin: Option<Origin>,
    pub config_origin: Option<Origin>,
    /// true: Marks the requirement to require manual verification.
    ///
    /// **Note:** All potential children of such a requirement are also marked
    /// to require manual verification.
    /// [req("req.manual")]
    pub manual_verification: RequirementAttributeState,
    /// true: Marks the requirement to be deprecated.
    ///
    /// **Note:** All potential children of such a requirement are also marked as deprecated.
    /// [req("req.deprecated")]
    pub deprecated: RequirementAttributeState,
    /// true: Instructs mantra to exclude the requirement for the product it is mapped to.
    ///
    /// **Note:** All potential children of such a requirement will also be excluded.
    /// [req("req.exclude")]
    pub excluded: RequirementAttributeState,
    pub draft: RequirementAttributeState,
    /// true: Instructs mantra to treat the requirement for the product as optional.
    ///
    /// **Note:** All potential children of such a requirement are also marked as optional.
    /// [req("req.optional")]
    pub optional: RequirementAttributeState,
    /// Optional list of requirements that this requirement replaces.
    pub replaces: Option<Vec<RequirementReference>>,
    /// Optional list of requirements that this requirement is replaced by.
    ///
    /// **Note:** If a requirement is replaced by others, it is automatically marked as deprecated.
    pub replaced_by: Option<Vec<RequirementReference>>,
    /// List of custom properties of a requirement.
    /// [req("req.properties")]
    pub properties: Option<Properties>,
    pub schema_properties: Option<Properties>,
    pub config_properties: Option<Properties>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequirementAttributeState {
    True,
    False,
    InheritedTrue,
    InheritedFalse,
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
    pub manual: bool,
}

impl RequirementReference {
    pub fn url_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Url)
    }

    pub fn os_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Os)
    }

    fn encode_path(&self, target: TargetEncoding) -> RelativePathBuf {
        let limit_id = crate::encoding::limit_str_len(&self.id);

        // Note: A limited id will be a base16 hash, and therefore not contain '::'
        let req_path = if limit_id.contains('.') {
            RelativePathBuf::from_iter(
                limit_id
                    .split('.')
                    .map(|id| crate::encoding::encode(id, target).to_string()),
            )
        } else {
            RelativePathBuf::from(crate::encoding::encode(&limit_id, target).to_string())
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
    Satisfied = 2,
    Skipped = 3,
    Deprecated = 4,
    Excluded = 5,
    /// Requirement may be directly or indirectly verified by children,
    /// but not all verified conditions have been fulfilled.
    PartiallyVerified = 6,
    Unknown = 7,
    Draft = 8,
}

impl RequirementState {
    pub fn as_number(&self) -> i32 {
        *self as i32
    }

    pub fn as_i64(&self) -> i64 {
        *self as i64
    }
}

impl TryFrom<i64> for RequirementState {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value == RequirementState::Failed.as_i64() {
            Ok(RequirementState::Failed)
        } else if value == RequirementState::Verified.as_i64() {
            Ok(RequirementState::Verified)
        } else if value == RequirementState::Satisfied.as_i64() {
            Ok(RequirementState::Satisfied)
        } else if value == RequirementState::Skipped.as_i64() {
            Ok(RequirementState::Skipped)
        } else if value == RequirementState::Deprecated.as_i64() {
            Ok(RequirementState::Deprecated)
        } else if value == RequirementState::Excluded.as_i64() {
            Ok(RequirementState::Excluded)
        } else if value == RequirementState::PartiallyVerified.as_i64() {
            Ok(RequirementState::PartiallyVerified)
        } else if value == RequirementState::Unknown.as_i64() {
            Ok(RequirementState::Unknown)
        } else if value == RequirementState::Draft.as_i64() {
            Ok(RequirementState::Draft)
        } else {
            Err(ConversionError::UnknownState)
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
pub struct RequirementCoverage {
    pub test: TestReference,
    pub covered_traces: Option<Vec<TraceReference>>,
    pub directly_verified: bool,
    /// `true` means that eventhough the test covers the requirement by a `verify` trace,
    /// it is not covering any `satisfy` trace that exists for the requirement.
    /// Therefore, this test does not verify the requirement via traces.
    pub missing_satisfy_trace: bool,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct RequirementReview {
    pub review: ReviewReference,
    pub rational: FormattableContent,
}
