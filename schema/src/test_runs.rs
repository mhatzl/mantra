use std::{path::PathBuf, time::Duration};

use crate::{requirements::ReqId, Line, Origin, Properties, Revision};

/// Defines the schema to exchange test and coverage related information.
/// [req("exchange.testcov.schema")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct TestRunSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    /// List of test runs containing test and coverage information.
    pub test_runs: Vec<TestRun>,
    /// Optional properties related to all test runs in this entry.
    ///
    /// **Note:** If a test run sets a property key directly,
    /// the value set at the test run will be taken.
    pub properties: Option<Properties>,
    /// Optional base origin of the test runs in this entry.
    /// e.g. specific branch or commit from a git repository
    pub origin: Option<Origin>,
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
    /// Optional revisions for the test run.
    pub revisions: Option<Vec<Revision>>,
    /// Optional origin of the test run.
    /// [req("testcov.test_run.origin")]
    pub origin: Option<Origin>,
    /// Nr of test cases that are part of the test run.
    ///
    /// **Note:** Must match with the number of entries in the `test_cases` field,
    /// plus the number of entries in the `test_cases` fields of all child test runs.
    /// In case this differs, it indicates that not all test cases have finished execution.
    #[serde(alias = "nr-of-tests")]
    pub nr_of_test_cases: u32,
    /// Optional field to store custom information per test run.
    /// [req("testcov.test_run.metadata")]
    pub properties: Option<Properties>,
    /// Optional duration about how long the test run took.
    pub duration: Option<Duration>,
    /// Optional logs that were output during the execution of the test run.
    ///
    // TODO: add req
    pub logs: Option<Vec<LogOutput>>,
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

/// Represents the primary key for a test run.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct TestRunPk {
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
    /// Optional field to store custom properties per test case.
    /// [req("testcov.test_case.metadata")]
    pub properties: Option<Properties>,
    /// Optional location of the test case.
    /// [req("testcov.test_case.origin")]
    pub location: Option<TestCaseLocation>,
    /// State of the test case.
    /// [req("testcov.test_case.state")]
    pub state: TestCaseState,
    /// Optional reason for the test case state.
    /// [req("testcov.test_case.state.reason")]
    pub state_properties: Option<Properties>,
    /// Optional logs that were output during the test case execution.
    // TODO: add req
    pub logs: Option<Vec<LogOutput>>,
    /// Optional requirements that were explicitely verified by the test case.
    #[serde(default)]
    pub verified_reqs: Vec<ReqId>,
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
    /// Optional hash of the file content to detect changes.
    /// Coverage formats may not provide the file hash, therefore it must be optional.
    pub file_hash: Option<String>,
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

/// Log output of tests.
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
pub struct LogOutput {
    /// The source the log was output to.
    pub source: LogSource,
    /// The log content that was output.
    pub content: String,
}

/// Source of the log output of tests.
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
pub enum LogSource {
    Stdout = 0,
    Stderr = 1,
}
