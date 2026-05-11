use crate::report::{product::ProductMetadata, test_run::TestRunReference, tests::TestsSummary};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TestRunsReportSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    pub product: ProductMetadata,
    pub test_cases_summary: TestsSummary,
    pub test_runs: TestRunsOverview,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TestRunsOverview {
    pub summary: TestsSummary,
    pub passed: Vec<TestRunReference>,
    pub failed: Vec<TestRunReference>,
    pub skipped: Vec<TestRunReference>,
    pub unknown: Vec<TestRunReference>,
    pub obsolete: Vec<TestRunReference>,
}
