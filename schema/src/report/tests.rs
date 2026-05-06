use relative_path::RelativePathBuf;

use crate::{
    ConversionError, FmtHash,
    product::ProductId,
    report::{
        Aggregated, annotations::TraceReference, test_case::TestCaseReference,
        test_run::TestRunReference,
    },
    requirements::ReqId,
};

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TestsSummary {
    pub total: i64,
    pub passed: Aggregated,
    pub failed: Aggregated,
    pub skipped: Aggregated,
    pub unknown: Aggregated,
    pub obsolete: Aggregated,
}

impl TestsSummary {
    pub fn add(&mut self, other: &Self) {
        self.total += other.total;

        self.passed.cnt += other.passed.cnt;
        self.failed.cnt += other.failed.cnt;
        self.skipped.cnt += other.skipped.cnt;
        self.unknown.cnt += other.unknown.cnt;
        self.obsolete.cnt += other.obsolete.cnt;

        self.update_percentages();
    }

    pub fn update_percentages(&mut self) {
        self.passed.update_percentage(self.total);
        self.failed.update_percentage(self.total);
        self.skipped.update_percentage(self.total);
        self.unknown.update_percentage(self.total);
        self.obsolete.update_percentage(self.total);
    }
}

/// Possible states a test run or test case may be in.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TestState {
    /// Test failed.
    Failed = 0,
    /// Test passed successfully.
    Passed = 1,
    /// Test was skipped in the related test run.
    Skipped = 2,
    /// Test is in an unknown state.
    ///
    /// This likely indicates that a test case did not finish execution,
    /// and is treated as *failed* state.
    /// [req("testcov.test_case.state.unknown")]
    Unknown = 3,
    /// Test is obsolete.
    ///
    /// Obsolete tests must not be considered for coverage analysis.
    Obsolete = 4,
}

impl TestState {
    pub fn as_nr(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<i64> for TestState {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value == TestState::Failed.as_nr() as i64 {
            Ok(TestState::Failed)
        } else if value == TestState::Skipped.as_nr() as i64 {
            Ok(TestState::Skipped)
        } else if value == TestState::Passed.as_nr() as i64 {
            Ok(TestState::Passed)
        } else if value == TestState::Unknown.as_nr() as i64 {
            Ok(TestState::Unknown)
        } else if value == TestState::Obsolete.as_nr() as i64 {
            Ok(TestState::Obsolete)
        } else {
            Err(ConversionError::UnknownState)
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TestCoverage {
    pub summary: TestCoverageSummary,
    pub covered_files: Vec<TestCoveredFile>,
    pub covered_traces: Option<Vec<TraceReference>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TestCoveredFile {
    #[schemars(with = "Vec<String>")]
    pub filepath: RelativePathBuf,
    pub fmt_hash: Option<FmtHash>,
}

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TestCoverageSummary {
    pub lines: CoverageSummary,
}

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct CoverageSummary {
    pub total: i64,
    pub covered: Aggregated,
    pub excluded: Aggregated,
    pub overridden: Aggregated,
    pub uncovered: Aggregated,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TestRelatedRequirement {
    pub product_id: ProductId,
    pub id: ReqId,
    pub kind: TestRelatedRequirementKind,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TestRelatedRequirementKind {
    Direct,
    Traced(Vec<TraceReference>),
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TestReference {
    TestRun(TestRunReference),
    TestCase(TestCaseReference),
}
