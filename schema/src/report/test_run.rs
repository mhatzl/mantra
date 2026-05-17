use relative_path::RelativePathBuf;

use crate::{
    Origin, Properties, Revision, TEST_RUNS_FOLDER_NAME,
    encoding::TargetEncoding,
    product::ProductId,
    report::{
        product::ProductMetadata,
        review::ReviewReference,
        test_case::TestCaseReference,
        tests::{TestCoverage, TestRelatedRequirement, TestState, TestsSummary},
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

impl TestRunReference {
    pub fn url_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Url)
    }

    pub fn os_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Os)
    }

    fn encode_path(&self, target: TargetEncoding) -> RelativePathBuf {
        let product_path = match target {
            TargetEncoding::Os => self.product_id.os_path(),
            TargetEncoding::Url => self.product_id.url_path(),
        };

        product_path.join(TEST_RUNS_FOLDER_NAME).join(format!(
            "{}_{}",
            super::encode_utc_date(&self.utc_date),
            crate::encoding::encode(&self.name, target)
        ))
    }
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
    pub state: TestState,
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
    pub nr_of_test_cases: i64,
    /// Optional field to store custom information per test run.
    /// [req("testcov.test_run.metadata")]
    pub properties: Option<Properties>,
    /// Optional duration about how long the test run took.
    /// Will be displayed in seconds with nanosecond precision in decimal form.
    #[schemars(with = "String")]
    #[serde(with = "crate::test_runs::duration_as_saturating_seconds_f64", default)]
    pub duration_sec: Option<time::Duration>,
    /// Optional logs that were output during the execution of the test run.
    ///
    // TODO: add req
    pub logs: Option<Vec<LogOutput>>,
    /// Overview of test cases that are part of the test run.
    /// [req("testcov.test_case")]
    pub test_cases: Option<TestCasesOverview>,
    /// Optional test run children.
    pub child_test_runs: Option<Vec<TestRunReference>>,
    /// Optional test run parents.
    pub parent_test_runs: Option<Vec<TestRunReference>>,
    /// Optional list of coverage information per file that was collected during the test run.
    /// [req("testcov.cov")]
    pub coverage: Option<TestCoverage>,
    pub related_reqs: Option<Vec<TestRelatedRequirement>>,
    pub overridden_by: Option<Vec<ReviewReference>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TestCasesOverview {
    pub summary: TestsSummary,
    pub passed: Vec<TestCaseReference>,
    pub failed: Vec<TestCaseReference>,
    pub skipped: Vec<TestCaseReference>,
    pub unknown: Vec<TestCaseReference>,
    pub obsolete: Vec<TestCaseReference>,
}
