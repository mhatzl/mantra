use relative_path::RelativePathBuf;

use crate::{
    Properties,
    encoding::TargetEncoding,
    product::ProductId,
    report::{
        product::ProductMetadata,
        review::ReviewReference,
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
    pub fn url_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Url)
    }

    pub fn os_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Os)
    }

    fn encode_path(&self, target: TargetEncoding) -> RelativePathBuf {
        let test_run = TestRunReference {
            product_id: self.product_id.clone(),
            name: self.test_run_name.clone(),
            utc_date: self.test_run_date,
            state: TestState::Unknown,
        };

        let limit_test_case_name = crate::encoding::limit_str_len(&self.test_case_name);

        // Note: A limited name will be a base16 hash, and therefore not contain '::'
        let test_case_path = if limit_test_case_name.contains("::") {
            RelativePathBuf::from_iter(
                limit_test_case_name
                    .split("::")
                    .map(|name| crate::encoding::encode(name, target).to_string()),
            )
        } else {
            RelativePathBuf::from(
                crate::encoding::encode(&limit_test_case_name, target).to_string(),
            )
        };

        let test_run_path = match target {
            TargetEncoding::Os => test_run.os_path(),
            TargetEncoding::Url => test_run.url_path(),
        };

        test_run_path.join(test_case_path)
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
    pub overridden_by: Option<Vec<ReviewReference>>,
}
