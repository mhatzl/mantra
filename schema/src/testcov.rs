use std::path::PathBuf;

use crate::Line;

/// Defines the schema to exchange test and coverage related information.
/// [req("exchange.testcov.schema")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct TestCovSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    /// List of test runs containing test and coverage information.
    pub test_runs: Vec<TestRun>,
}

/// Represents a test run in *mantra*.
/// [req("testcov.test_run")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct TestRun {
    /// The name of the test run.
    /// [req("testcov.test_run.id")]
    pub name: String,
    /// The UTC date the test run execution started.
    ///
    /// **Note:** The date must be given in ISO8601 format.
    /// [req("testcov.test_run.date")]
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    /// Hash of the test run content to detect changes.
    ///
    /// If not provided, will be computed using the fields: origin, nr_of_test_cases, data, logs, test_cases, covered_files, test_runs
    /// [req("changes.track.test_runs")]
    pub content_hash: Option<String>,
    /// Optional origin of the test run.
    /// [req("testcov.test_run.origin")]
    pub origin: Option<serde_json::Value>,
    /// Nr of test cases that are part of the test run.
    ///
    /// **Note:** Must match with the number of entries in the `test_cases` field,
    /// plus the number of entries in the `test_cases` fields of all child test runs.
    /// In case this differs, it indicates that not all test cases have finished execution.
    #[serde(alias = "nr-of-tests")]
    pub nr_of_test_cases: u32,
    /// Optional VCS identifier for the content the test run is based on (e.g. git commit SHA).
    /// [req("testcov.cov.trace_mapping.vcs")]
    pub vcs_ident: Option<String>,
    /// Optional field to store custom information per test run.
    /// [req("testcov.test_run.metadata")]
    pub data: Option<serde_json::Value>,
    /// Optional logs that were output during the execution of the test run.
    ///
    // TODO: add req
    pub logs: Option<String>,
    /// List of test cases that are part of the test run.
    /// [req("testcov.test_case")]
    #[serde(alias = "tests")]
    pub test_cases: Vec<TestCase>,
    /// Optional list of coverage information per file that was collected during the test run.
    /// [req("testcov.cov")]
    #[serde(default)]
    pub covered_files: Vec<CoveredFile>,
    /// Optionally nested test runs.
    /// [req("testcov.test_run.nested")]
    #[serde(default)]
    pub test_runs: Vec<TestRun>,
}

/// Represents the unique identifier for a test run.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct TestRunId {
    /// The name of the test run.
    /// [req("testcov.test_run.id")]
    pub name: String,
    /// The UTC date the test run execution started.
    ///
    /// **Note:** The date must be given in ISO8601 format.
    /// [req("testcov.test_run.date")]
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    /// Indicates the revision of a test run to track retrospective changes.
    /// [req("changes.track.test_runs")]
    pub revision: usize,
}

/// Represents a test case in *mantra*.
/// [req("testcov.test_case")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct TestCase {
    /// The name of the test case.
    /// [req("testcov.test_case.id")]
    pub name: String,
    /// Optional location of the test case.
    /// [req("testcov.test_case.origin")]
    pub location: Option<TestCaseLocation>,
    /// State of the test case.
    /// [req("testcov.test_case.state")]
    pub state: TestCaseState,
    /// Optional reason for the test case state.
    /// [req("testcov.test_case.state.reason")]
    pub state_reason: Option<String>,
    /// Optional field to store custom information per test case.
    /// [req("testcov.test_case.metadata")]
    pub data: Option<serde_json::Value>,
    /// Optional logs that were output during the test case execution.
    // TODO: add req
    pub logs: Option<String>,
    /// Optional list of coverage data per file that was collected during the execution of the test case.
    /// [req("testcov.cov")]
    #[serde(default)]
    pub covered_files: Vec<CoveredFile>,
}

/// The location of a test case.
/// [req("testcov.test_case.origin")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct TestCaseLocation {
    /// The filepath the test case is defined in.
    pub filepath: PathBuf,
    /// The hash of the file content at the time the test case was executed.
    /// [req("changes.track.test_runs")]
    pub file_hash: Option<String>,
    /// The line in the file the test case is defined at.
    pub line: Line,
}

/// Possible states a test case may be in.
/// [req("testcov.test_case.state")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum TestCaseState {
    /// Test case failed.
    Failed = 0,
    /// Test case passed successfully.
    Passed = 1,
    /// Test case was skipped in the related test run.
    Skipped = 2,
    /// Test case is in an unknown state.
    ///
    /// This likely indicates that a test case did not finish execution,
    /// and is treated as *failed* state.
    /// [req("testcov.test_case.state.unknown")]
    Unknown = 3,
}

/// Represents coverage information per file.
/// [req("testcov.cov")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct CoveredFile {
    /// File the coverage information is for.
    pub filepath: PathBuf,
    /// Coverage information for a line in the file.
    /// [req("testcov.cov.lines")]
    #[serde(default)]
    pub lines: Vec<CoveredLine>,
}

/// Coverage information of a line in a file.
/// [req("testcov.cov.lines")]
#[derive(
    Debug,
    Clone,
    PartialEq,
    PartialOrd,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub struct CoveredLine {
    /// The line number.
    pub nr: Line,
    /// The number of times this line has been reached during execution of a test run or test case.
    pub hits: usize,
}

impl std::cmp::Ord for CoveredLine {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.nr.cmp(&other.nr)
    }
}
