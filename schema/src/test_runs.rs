use time::{Duration, OffsetDateTime};

use crate::path::RelativePathBuf;
use crate::{ConversionError, FmtHash};
use crate::{Line, Origin, Properties, Revision, requirements::ReqId};

/// Tries to convert the given string to an [`OffsetDateTime`]
/// using the [`Iso8601`](time::format_description::well_known::Iso8601) format.
pub fn test_date_from_str(date: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(
        date,
        &time::format_description::well_known::Iso8601::PARSING,
    )
}

/// Defines the schema to exchange test and coverage related information.
/// [req("exchange.testcov.schema")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TestRunSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    /// List of test runs containing test and coverage information.
    pub test_runs: Vec<TestRun>,
    /// Optional properties related to all test runs in this entry.
    ///
    /// **Note:** If a test run sets a property key directly,
    /// the value set at the test run will be taken.
    pub test_run_properties: Option<Properties>,
    /// Optional properties related to all test cases in this entry.
    ///
    /// **Note:** If a test case sets a property key directly,
    /// the value set at the test case will be taken.
    pub test_case_properties: Option<Properties>,
    /// Optional base origin of the test runs in this entry.
    /// e.g. specific branch or commit from a git repository
    pub origin: Option<Origin>,
}

/// Represents a test run in *mantra*.
/// [req("testcov.test_run")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TestRun {
    /// The name of the test run.
    /// [req("testcov.test_run.id")]
    pub name: String,
    /// The UTC date the test run execution started.
    ///
    /// **Note:** The date must be given in ISO8601 format.
    /// [req("testcov.test_run.date")]
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    /// Optional description of the test run.
    pub description: Option<String>,
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
    #[serde(alias = "nr_of_tests")]
    pub nr_of_test_cases: u32,
    /// Optional field to store custom information per test run.
    /// [req("testcov.test_run.metadata")]
    pub properties: Option<Properties>,
    /// Optional duration about how long the test run took.
    #[schemars(with = "Option<f64>")]
    #[serde(with = "duration_as_saturating_seconds_f64", default)]
    pub duration_sec: Option<Duration>,
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

/// Represents a test case in *mantra*.
/// [req("testcov.test_case")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TestCase {
    /// The name of the test case.
    /// [req("testcov.test_case.id")]
    pub name: String,
    /// Optional description of the test case.
    pub description: Option<String>,
    /// State of the test case.
    /// [req("testcov.test_case.state")]
    pub state: TestCaseState,
    /// Optional reason for the test case state.
    /// [req("testcov.test_case.state.reason")]
    pub state_properties: Option<Properties>,
    /// Optional location of the test case.
    /// [req("testcov.test_case.origin")]
    pub location: Option<TestCaseLocation>,
    /// Optional UTC date the test case execution started.
    ///
    /// **Note:** The date must be given in ISO8601 format.
    #[serde(with = "time::serde::iso8601::option")]
    #[schemars(with = "String")]
    #[serde(default)] // Needed due to: https://github.com/serde-rs/serde/issues/2878
    pub utc_date: Option<time::OffsetDateTime>,
    /// Optional duration about how long the test case took.
    #[schemars(with = "Option<f64>")]
    #[serde(with = "duration_as_saturating_seconds_f64", default)]
    pub duration_sec: Option<Duration>,
    /// Optional field to store custom properties per test case.
    /// [req("testcov.test_case.metadata")]
    pub properties: Option<Properties>,
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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TestCaseLocation {
    /// The filepath the test case is defined in.
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    /// The hash of the file content at the time the test case was executed.
    /// [req("changes.track.test_runs")]
    #[schemars(with = "String")]
    pub file_hash: Option<FmtHash>,
    /// The line in the file the test case is defined at.
    pub line: Line,
}

/// Possible states a test case may be in.
/// [req("testcov.test_case.state")]
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

impl TestCaseState {
    pub fn as_nr(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<i64> for TestCaseState {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value == TestCaseState::Failed.as_nr() as i64 {
            Ok(TestCaseState::Failed)
        } else if value == TestCaseState::Skipped.as_nr() as i64 {
            Ok(TestCaseState::Skipped)
        } else if value == TestCaseState::Passed.as_nr() as i64 {
            Ok(TestCaseState::Passed)
        } else if value == TestCaseState::Unknown.as_nr() as i64 {
            Ok(TestCaseState::Unknown)
        } else {
            Err(ConversionError::UnknownState)
        }
    }
}

/// Represents coverage information per file.
/// [req("testcov.cov")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CoveredFile {
    /// File the coverage information is for.
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    /// Optional hash of the file content to detect changes.
    /// Coverage formats may not provide the file hash, therefore it must be optional.
    pub file_hash: Option<FmtHash>,
    /// Coverage information for a line in the file.
    /// [req("testcov.cov.lines")]
    #[serde(default)]
    pub lines: Vec<CoveredLine>,
}

/// Coverage information of a line in a file.
/// [req("testcov.cov.lines")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct CoveredLine {
    /// The line number.
    pub nr: Line,
    /// The number of times this line has been reached during execution of a test run or test case.
    /// If None, the line is marked to be ignored from line coverage analysis.
    ///
    /// **Note:** The line might be covered by other test runs or test cases.
    /// To permanently exclude lines, see the AnnotationSchema.
    pub hits: Option<i64>,
}

impl std::cmp::PartialOrd for CoveredLine {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for CoveredLine {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.nr.cmp(&other.nr) {
            std::cmp::Ordering::Equal => self.hits.cmp(&other.hits),
            cmp => cmp,
        }
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
#[serde(deny_unknown_fields)]
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
    Copy,
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

impl LogSource {
    pub fn as_nr(&self) -> i32 {
        *self as i32
    }
}

impl std::fmt::Display for LogSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogSource::Stdout => write!(f, "stdout"),
            LogSource::Stderr => write!(f, "stderr"),
        }
    }
}

impl TryFrom<i64> for LogSource {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value == LogSource::Stdout.as_nr() as i64 {
            Ok(LogSource::Stdout)
        } else if value == LogSource::Stderr.as_nr() as i64 {
            Ok(LogSource::Stderr)
        } else {
            Err(ConversionError::UnknownState)
        }
    }
}

pub mod duration_as_saturating_seconds_f64 {
    use serde::de::Visitor;
    use time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match duration {
            Some(value) => serializer.serialize_f64(value.as_seconds_f64()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionDurationVisitor)
    }

    struct OptionDurationVisitor;

    impl<'de> Visitor<'de> for OptionDurationVisitor {
        type Value = Option<Duration>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "optional duration as f64 seconds.")
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_f64(DurationVisitor).map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    struct DurationVisitor;

    impl<'de> Visitor<'de> for DurationVisitor {
        type Value = Duration;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "duration as f64 seconds.")
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(time::Duration::saturating_seconds_f64(v))
        }
    }
}
