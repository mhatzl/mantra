use crate::{
    Origin, Properties, Revision,
    product::ProductId,
    report::{
        product::ProductMetadata,
        test_case::TestCaseReference,
        tests::{TestCoverage, TestRelatedRequirement, TestState},
    },
    test_runs::LogOutput,
};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TestRunReference {
    pub product_id: ProductId,
    pub name: String,
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    pub state: TestState,
}

/// Represents a test run in *mantra*.
/// [req("testcov.test_run")]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TestRunReportSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    pub product: ProductMetadata,
    /// The name of the test run.
    /// [req("testcov.test_run.id")]
    pub name: String,
    /// The UTC date the test run execution started.
    ///
    /// **Note:** The date must be given in ISO8601 format.
    /// [req("testcov.test_run.date")]
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    /// Optional description of the test run.
    pub description: Option<String>,
    /// Optional revisions for the test run.
    pub revisions: Option<Vec<Revision>>,
    /// Optional origin of the test run.
    /// [req("testcov.test_run.origin")]
    pub origin: Option<Origin>,
    pub base_origin: Option<Origin>,
    /// Nr of test cases that are part of the test run.
    ///
    /// **Note:** Must match with the number of entries in the `test_cases` field,
    /// plus the number of entries in the `test_cases` fields of all child test runs.
    /// In case this differs, it indicates that not all test cases have finished execution.
    pub nr_of_test_cases: u32,
    /// Optional field to store custom information per test run.
    /// [req("testcov.test_run.metadata")]
    pub properties: Option<Properties>,
    /// Optional duration about how long the test run took.
    /// Will be displayed in seconds with nanosecond precision in decimal form.
    #[schemars(with = "String")]
    pub duration: Option<time::Duration>,
    /// Optional logs that were output during the execution of the test run.
    ///
    // TODO: add req
    pub logs: Option<Vec<LogOutput>>,
    /// List of test cases that are part of the test run.
    /// [req("testcov.test_case")]
    pub test_cases: Option<Vec<TestCaseReference>>,
    /// Optional list of coverage information per file that was collected during the test run.
    /// [req("testcov.cov")]
    pub coverage: Option<TestCoverage>,
    /// Optionally nested test runs.
    /// [req("testcov.test_run.nested")]
    pub test_runs: Option<Vec<TestRunReportSchema>>,
    pub related_reqs: Option<Vec<TestRelatedRequirement>>,
}
