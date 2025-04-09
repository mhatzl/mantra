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

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TestRun {
    pub name: String,
    /// Test run date must be given in ISO8601 format.
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    #[schemars(with = "String")]
    pub date: time::OffsetDateTime,
    /// Hash of the test run content to detect changes.
    #[serde(alias = "content-hash")]
    pub content_hash: Option<String>,
    /// ISO8601 timestamp when the test run was last checked.
    #[serde(
        alias = "last-checked-at",
        serialize_with = "time::serde::iso8601::option::serialize",
        deserialize_with = "time::serde::iso8601::option::deserialize"
    )]
    #[schemars(with = "Option<String>")]
    pub last_checked_at: Option<time::OffsetDateTime>,
    #[serde(alias = "nr-of-tests")]
    pub nr_of_tests: u32,
    /// Field to store custom information per test run.
    pub data: Option<serde_json::Value>,
    pub logs: Option<String>,
    pub tests: Vec<Test>,
    #[serde(default, alias = "covered-files")]
    pub covered_files: Vec<CoveredFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestRunPk {
    pub name: String,
    pub date: time::OffsetDateTime,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TestLocation {
    pub filepath: PathBuf,
    pub line: Line,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Test {
    pub name: String,
    pub location: Option<TestLocation>,
    pub state: TestState,
    #[serde(default, alias = "covered-files")]
    pub covered_files: Vec<CoveredFile>,
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

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum TestState {
    Passed,
    Failed,
    Skipped { reason: Option<String> },
}
