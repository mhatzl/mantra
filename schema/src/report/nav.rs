use crate::report::{
    product::ProductMetadata, requirement::RequirementReference, review::ReviewReference,
    sources::SourceReference, test_case::TestCaseReference, test_run::TestRunReference,
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
    pub root_sources: Vec<SourceReference>,
}

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ProductNavigation {
    pub product: ProductMetadata,
    pub root_requirements: Vec<RequirementReference>,
    pub root_test_runs: Vec<TestRunNavigation>,
    pub reviews: Vec<ReviewReference>,
}

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TestRunNavigation {
    pub name: String,
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    pub test_runs: Vec<TestRunReference>,
    pub test_cases: Vec<TestCaseReference>,
}
