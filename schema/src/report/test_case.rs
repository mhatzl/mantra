use crate::{
    Properties,
    product::ProductId,
    report::{
        product::ProductMetadata,
        test_run::TestRunReference,
        tests::{TestCoverage, TestRelatedRequirement, TestState},
    },
    test_runs::{LogOutput, TestCaseLocation},
};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TestCaseReference {
    pub product_id: ProductId,
    pub test_run_name: String,
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub test_run_date: time::OffsetDateTime,
    pub test_case_name: String,
    pub state: TestState,
}

impl TestCaseReference {
    pub fn url_path_part(&self) -> String {
        urlencoding::encode(&self.test_case_name).to_string()
    }
}

/// Represents a test case in *mantra*.
/// [req("testcov.test_case")]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TestCaseReportSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    pub product: ProductMetadata,
    pub test_run: TestRunReference,
    /// The name of the test case.
    /// [req("testcov.test_case.id")]
    pub name: String,
    /// Optional description of the test case.
    pub description: Option<String>,
    /// State of the test case.
    /// [req("testcov.test_case.state")]
    pub state: TestState,
    /// Optional reason for the test case state.
    /// [req("testcov.test_case.state.reason")]
    pub state_properties: Option<Properties>,
    /// Optional location of the test case.
    /// [req("testcov.test_case.origin")]
    pub location: Option<TestCaseLocation>,
    /// Optional UTC date the test case execution started.
    ///
    /// **Note:** The date must be given in ISO8601 format.
    #[serde(with = "time::serde::iso8601::option")]
    #[schemars(with = "String")]
    #[serde(default)] // Needed due to: https://github.com/serde-rs/serde/issues/2878
    pub utc_date: Option<time::OffsetDateTime>,
    /// Optional duration about how long the test case took.
    #[schemars(with = "String")]
    #[serde(with = "crate::test_runs::duration_as_saturating_seconds_f64", default)]
    pub duration_sec: Option<time::Duration>,
    /// Optional field to store custom properties per test case.
    /// [req("testcov.test_case.metadata")]
    pub properties: Option<Properties>,
    /// Optional logs that were output during the test case execution.
    // TODO: add req
    pub logs: Option<Vec<LogOutput>>,
    /// Optional list of coverage data per file that was collected during the execution of the test case.
    /// [req("testcov.cov")]
    pub coverage: Option<TestCoverage>,
    pub related_reqs: Option<Vec<TestRelatedRequirement>>,
}
