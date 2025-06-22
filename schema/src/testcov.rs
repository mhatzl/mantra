use std::path::PathBuf;

use crate::Line;

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TestCovSchema {
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    #[serde(alias = "test-runs")]
    pub test_runs: Vec<TestRun>,
}

/// Represents a test run in *mantra*.
/// [req("testcov.test_run")]
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct TestRun {
    pub name: String,
    /// Test run date must be given in ISO8601 format.
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    /// Hash of the test run content to detect changes.
    ///
    /// If not provided, will be computed using the fields: nr_of_test_cases, data, logs, test_cases, covered_files
    pub content_hash: Option<String>,
    #[serde(alias = "nr-of-tests")]
    pub nr_of_test_cases: u32,
    /// Field to store custom information per test run.
    pub data: Option<serde_json::Value>,
    pub logs: Option<String>,
    #[serde(alias = "tests")]
    pub test_cases: Vec<TestCase>,
    #[serde(default)]
    pub covered_files: Vec<CoveredFile>,
    /// Optionally nested test runs.
    /// [req("testcov.test_run.nested")]
    #[serde(default)]
    pub test_runs: Vec<TestRun>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct TestCase {
    pub name: String,
    pub location: Option<TestCaseLocation>,
    pub state: TestCaseState,
    pub state_reason: Option<String>,
    /// Field to store custom information per test case.
    pub data: Option<serde_json::Value>,
    pub logs: Option<String>,
    #[serde(default)]
    pub covered_files: Vec<CoveredFile>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct TestCaseLocation {
    pub filepath: PathBuf,
    pub file_hash: Option<String>,
    pub line: Line,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum TestCaseState {
    Failed = 0,
    Passed = 1,
    Skipped = 2,
    Unknown = 3,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct CoveredFile {
    pub filepath: PathBuf,
    #[serde(default)]
    pub statements: Vec<CoveredStatement>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct CoveredStatement {
    pub line: Line,
    pub hits: usize,
}

impl std::cmp::PartialOrd for CoveredStatement {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for CoveredStatement {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.line.cmp(&other.line)
    }
}
