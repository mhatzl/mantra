use relative_path::RelativePathBuf;

use crate::{
    ConversionError, FmtHash,
    report::{
        Aggregated, annotations::TraceReference, requirement::RequirementReference,
        test_case::TestCaseReference, test_run::TestRunReference,
    },
    test_runs::TestCaseState,
};

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
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

/// ensure that the test states remain identically numbered
const _: () = {
    assert!(
        TestState::Failed.as_number() == TestCaseState::Failed.as_number()
            && TestState::Passed.as_number() == TestCaseState::Passed.as_number()
            && TestState::Skipped.as_number() == TestCaseState::Skipped.as_number()
            && TestState::Unknown.as_number() == TestCaseState::Unknown.as_number()
    )
};

impl TestState {
    pub const fn as_number(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<i64> for TestState {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value == TestState::Failed.as_number() as i64 {
            Ok(TestState::Failed)
        } else if value == TestState::Skipped.as_number() as i64 {
            Ok(TestState::Skipped)
        } else if value == TestState::Passed.as_number() as i64 {
            Ok(TestState::Passed)
        } else if value == TestState::Unknown.as_number() as i64 {
            Ok(TestState::Unknown)
        } else if value == TestState::Obsolete.as_number() as i64 {
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
    pub file_hash: Option<FmtHash>,
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
    pub uncovered: Aggregated,
    pub excluded: Aggregated,
    pub overridden_covered: Aggregated,
    pub overridden_uncovered: Aggregated,
    pub overridden_excluded: Aggregated,
    pub unknown: Aggregated,
}

impl CoverageSummary {
    pub fn add(&mut self, other: &Self) {
        self.total += other.total;

        self.covered.cnt += other.covered.cnt;
        self.uncovered.cnt += other.uncovered.cnt;
        self.excluded.cnt += other.excluded.cnt;
        self.overridden_covered.cnt += other.overridden_covered.cnt;
        self.overridden_uncovered.cnt += other.overridden_uncovered.cnt;
        self.overridden_excluded.cnt += other.overridden_excluded.cnt;
        self.unknown.cnt += other.unknown.cnt;

        self.update_percentages();
    }

    pub fn update_percentages(&mut self) {
        self.covered.update_percentage(self.total);
        self.uncovered.update_percentage(self.total);
        self.excluded.update_percentage(self.total);
        self.overridden_covered.update_percentage(self.total);
        self.overridden_uncovered.update_percentage(self.total);
        self.overridden_excluded.update_percentage(self.total);
        self.unknown.update_percentage(self.total);
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TestRelatedRequirement {
    pub req: RequirementReference,
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
pub enum ResolvedCoverageState {
    Uncovered = 0,
    Covered = 1,
    /// Coverage excluded by at least one exclude annotation.
    Excluded = 2,
    /// Indicates a section is not part of the resulting executable.
    /// e.g code behind disabled feature flags
    Unknown = 3,
    OverriddenUncovered = 4,
    OverriddenCovered = 5,
    /// Coverage explicitly excluded by at least one valid review.
    OverriddenExcluded = 6,
}

impl ResolvedCoverageState {
    pub fn as_number(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<i64> for ResolvedCoverageState {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value == ResolvedCoverageState::Uncovered.as_number() as i64 {
            Ok(ResolvedCoverageState::Uncovered)
        } else if value == ResolvedCoverageState::Covered.as_number() as i64 {
            Ok(ResolvedCoverageState::Covered)
        } else if value == ResolvedCoverageState::Excluded.as_number() as i64 {
            Ok(ResolvedCoverageState::Excluded)
        } else if value == ResolvedCoverageState::Unknown.as_number() as i64 {
            Ok(ResolvedCoverageState::Unknown)
        } else if value == ResolvedCoverageState::OverriddenUncovered.as_number() as i64 {
            Ok(ResolvedCoverageState::OverriddenUncovered)
        } else if value == ResolvedCoverageState::OverriddenCovered.as_number() as i64 {
            Ok(ResolvedCoverageState::OverriddenCovered)
        } else if value == ResolvedCoverageState::OverriddenExcluded.as_number() as i64 {
            Ok(ResolvedCoverageState::OverriddenExcluded)
        } else {
            Err(ConversionError::UnknownState)
        }
    }
}
