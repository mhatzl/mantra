use std::path::PathBuf;

use crate::{requirements::ReqId, Line};

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct CoverageSchema {
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
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
    pub nr_of_tests: u32,
    pub meta: Option<serde_json::Value>,
    pub logs: Option<String>,
    pub tests: Vec<Test>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestRunPk {
    pub name: String,
    pub date: time::OffsetDateTime,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Test {
    pub name: String,
    pub filepath: PathBuf,
    pub line: Line,
    pub state: TestState,
    #[serde(default)]
    pub covered_files: Vec<CoveredFile>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct CoveredFile {
    pub filepath: PathBuf,
    #[serde(default)]
    pub covered_traces: Vec<CoveredFileTrace>,
    #[serde(default)]
    pub covered_lines: Vec<CoveredLine>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct CoveredFileTrace {
    pub req_ids: Vec<ReqId>,
    pub line: Line,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct CoveredLine {
    pub line: Line,
    pub hits: usize,
}

impl std::cmp::PartialOrd for CoveredLine {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for CoveredLine {
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
