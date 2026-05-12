use crate::{
    product::ProductId,
    report::{
        product::ProductMetadata,
        requirement::{RequirementReference, RequirementState},
        review::ReviewReference,
        test_case::TestCaseReference,
        test_run::TestRunReference,
    },
    requirements::ReqId,
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
    pub product_id: ProductId,
    pub id: ReqId,
    pub title: String,
    pub state: RequirementState,
    pub optional: bool,
    pub manual_verification: bool,
    pub parents: Option<Vec<RequirementReference>>,
    pub children: Option<Vec<RequirementReference>>,
    pub covered_by: Option<RequirementCoverageByTestsOverview>,
    pub reviewed_in: Option<Vec<ReviewReference>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RequirementCoverageByTestsOverview {
    pub test_runs: Vec<TestRunReference>,
    pub test_cases: Vec<TestCaseReference>,
}
