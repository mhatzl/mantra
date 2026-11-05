use mantra_schema::FmtHash;
use mantra_schema::Line;
use mantra_schema::path::RelativePathBuf;
use mantra_schema::requirements::ReqId;
use mantra_schema::test_runs::TestCaseState;

pub mod cfg;
pub mod changes;
pub mod collect;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DbIgnoredEntry {
    Requirement {
        id: ReqId,
        rational_hash: FmtHash,
    },
    TestCaseStateOverride {
        test_run_name: String,
        test_case_name: String,
        state: TestCaseState,
        rational_hash: FmtHash,
    },
    TestCaseLineCoverageOverride {
        test_run_name: String,
        test_case_name: String,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        covered: Option<bool>,
        rational_hash: FmtHash,
    },
    TestRunLineCoverageOverride {
        test_run_name: String,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        covered: Option<bool>,
        rational_hash: FmtHash,
    },
}

impl DbIgnoredEntry {
    fn from_verified_req(req_id: ReqId, rational_hash: FmtHash) -> Self {
        Self::Requirement {
            id: req_id,
            rational_hash,
        }
    }

    fn from_test_case_state(
        test_run_name: String,
        test_case_name: String,
        state: TestCaseState,
        rational_hash: FmtHash,
    ) -> Self {
        Self::TestCaseStateOverride {
            test_run_name,
            test_case_name,
            state,
            rational_hash,
        }
    }

    fn from_test_case_line_coverage(
        test_run_name: String,
        test_case_name: String,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        covered: Option<bool>,
        rational_hash: FmtHash,
    ) -> Self {
        Self::TestCaseLineCoverageOverride {
            test_run_name,
            test_case_name,
            cov_filepath,
            cov_line,
            covered,
            rational_hash,
        }
    }

    fn from_test_run_line_coverage(
        test_run_name: String,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        covered: Option<bool>,
        rational_hash: FmtHash,
    ) -> Self {
        Self::TestRunLineCoverageOverride {
            test_run_name,
            cov_filepath,
            cov_line,
            covered,
            rational_hash,
        }
    }
}
