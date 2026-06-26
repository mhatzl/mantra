use crate::report::{
    product::ProductMetadata, requirement::RequirementReference, review::ReviewReference,
    sources::SourceReference, test_run::TestRunReference,
};

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ReportNavigationSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    pub products: Vec<ProductNavigation>,
    pub root_sources: Vec<SourceNavigation>,
}

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ProductNavigation {
    pub product: ProductMetadata,
    pub root_requirements: Vec<RequirementReference>,
    pub root_test_runs: Vec<TestRunReference>,
    pub reviews: Vec<ReviewReference>,
}

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct SourceNavigation {
    pub source: SourceReference,
    pub folder: Option<Vec<SourceNavigation>>,
    pub files: Option<Vec<SourceReference>>,
}
