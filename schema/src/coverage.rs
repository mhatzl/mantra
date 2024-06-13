use std::path::PathBuf;

use mantra_lang_tracing::Line;

use super::traces::TracePk;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CoverageSchema {
    pub test_runs: Vec<TestRun>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TestRun {
    pub name: String,
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Test {
    pub name: String,
    pub filepath: PathBuf,
    pub line: Line,
    pub state: TestState,
    #[serde(default)]
    pub covered_traces: Vec<TracePk>,
    #[serde(default)]
    pub covered_lines: Vec<LineCoverage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LineCoverage {
    pub filepath: PathBuf,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TestState {
    Passed,
    Failed,
    Skipped { reason: Option<String> },
}
