use mantra_schema::FmtHash;
use mantra_schema::Line;
use mantra_schema::path::RelativePathBuf;
use mantra_schema::requirements::ReqId;
use mantra_schema::test_runs::TestCaseState;
use mantra_schema::time::OffsetDateTime;

pub mod cfg;
pub mod collect;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DbIgnoredEntry {
    Requirement {
        id: ReqId,
        comment_hash: FmtHash,
    },
    TestCaseStateOverride {
        test_run_name: String,
        test_run_date: OffsetDateTime,
        test_case_name: String,
        state: TestCaseState,
        comment_hash: FmtHash,
    },
    TestCaseLineCoverageOverride {
        test_run_name: String,
        test_run_date: OffsetDateTime,
        test_case_name: String,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    },
    TestRunLineCoverageOverride {
        test_run_name: String,
        test_run_date: OffsetDateTime,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    },
}

impl DbIgnoredEntry {
    fn from_verified_req(req_id: ReqId, comment_hash: FmtHash) -> Self {
        Self::Requirement {
            id: req_id,
            comment_hash,
        }
    }

    fn from_test_case_state(
        test_run_name: String,
        test_run_date: OffsetDateTime,
        test_case_name: String,
        state: TestCaseState,
        comment_hash: FmtHash,
    ) -> Self {
        Self::TestCaseStateOverride {
            test_run_name,
            test_run_date,
            test_case_name,
            state,
            comment_hash,
        }
    }

    fn from_test_case_line_coverage(
        test_run_name: String,
        test_run_date: OffsetDateTime,
        test_case_name: String,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    ) -> Self {
        Self::TestCaseLineCoverageOverride {
            test_run_name,
            test_run_date,
            test_case_name,
            cov_filepath,
            cov_line,
            hits,
            comment_hash,
        }
    }

    fn from_test_run_line_coverage(
        test_run_name: String,
        test_run_date: OffsetDateTime,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    ) -> Self {
        Self::TestRunLineCoverageOverride {
            test_run_name,
            test_run_date,
            cov_filepath,
            cov_line,
            hits,
            comment_hash,
        }
    }
}
