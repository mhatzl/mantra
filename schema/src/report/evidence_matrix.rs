use crate::report::{
    formattable_content::FormattableContent,
    product::ProductMetadata,
    requirement::{RequirementCoverage, RequirementReference, RequirementReview},
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct EvidenceMatrixSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    pub product: ProductMetadata,
    pub requirements: Vec<RequirementEvidence>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RequirementEvidence {
    pub req_ref: RequirementReference,
    pub title: FormattableContent,
    pub parents: Option<Vec<RequirementReference>>,
    pub children: Option<Vec<RequirementReference>>,
    pub covered_by: Option<Vec<RequirementCoverage>>,
    pub reviewed_in: Option<Vec<RequirementReview>>,
}
