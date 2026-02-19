use std::path::Path;

use chrono::FixedOffset;
use mantra_schema::{
    Origin, Properties, Revision,
    path::{PathExt, RelativePathBuf},
    test_runs::{CoveredFile, CoveredLine, LogOutput, TestCase, TestCaseState, TestRun},
    time::{self, Duration, OffsetDateTime},
};
use quick_junit::{Report, TestCaseStatus, TestSuite};

use crate::cmd::collect::cfg::{WellKnownCoverageFormat, WellKnownTestFormat};

/// Shallow variant of a test run that may not yet have a utc-date set.
/// Used to convert well-known test formats and uses the timestamp of the first linked coverage data
/// in case the utc-date was None.
pub struct ShallowTestRun {
    /// The name of the test run.
    /// [req("testcov.test_run.id")]
    pub name: String,
    /// The UTC date the test run execution started.
    /// [req("testcov.test_run.date")]
    pub utc_date: Option<time::OffsetDateTime>,
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
    pub nr_of_test_cases: u32,
    /// Optional field to store custom information per test run.
    /// [req("testcov.test_run.metadata")]
    pub properties: Option<Properties>,
    /// Optional duration about how long the test run took.
    /// Will be displayed in seconds with nanosecond precision in decimal form.
    pub duration: Option<Duration>,
    /// Optional logs that were output during the execution of the test run.
    ///
    // TODO: add req
    pub logs: Option<Vec<LogOutput>>,
    /// List of test cases that are part of the test run.
    /// [req("testcov.test_case")]
    pub test_cases: Vec<TestCase>,
    /// Optionally nested test runs.
    /// [req("testcov.test_run.nested")]
    pub test_runs: Vec<ShallowTestRun>,
}

impl ShallowTestRun {
    pub fn to_test_run(
        self,
        covered_files: Vec<CoveredFile>,
        coverage_timestamp: Option<OffsetDateTime>,
    ) -> TestRun {
        let utc_date = match self.utc_date.or(coverage_timestamp) {
            Some(timestamp) => timestamp,
            None => {
                // TODO: proper logging
                eprintln!("No timestamp collected. Using local timestamp.");
                OffsetDateTime::now_utc()
            }
        };

        TestRun {
            name: self.name,
            utc_date,
            description: self.description,
            revisions: self.revisions,
            origin: self.origin,
            nr_of_test_cases: self.nr_of_test_cases,
            properties: self.properties,
            duration: self.duration,
            logs: self.logs,
            test_cases: self.test_cases,
            covered_files,
            test_runs: self
                .test_runs
                .into_iter()
                .map(|t| t.to_test_run(vec![], Some(utc_date)))
                .collect(),
        }
    }
}

pub trait WellKnownTestConversion {
    fn to_shallow_test_run(
        &self,
        root_path: &Path,
        extension: &str,
        content: &str,
    ) -> Result<ShallowTestRun, anyhow::Error>;
}

impl WellKnownTestConversion for WellKnownTestFormat {
    fn to_shallow_test_run(
        &self,
        root_path: &Path,
        extension: &str,
        content: &str,
    ) -> Result<ShallowTestRun, anyhow::Error> {
        match self {
            WellKnownTestFormat::Junit => {
                if extension == "xml" {
                    let junit_report = quick_junit::Report::deserialize_from_str(content)?;
                    Ok(junit_to_shallow_test_run(junit_report, root_path)?)
                } else {
                    anyhow::bail!(
                        "Got unsupported test format extension for JUnit '{}'",
                        extension
                    )
                }
            }
        }
    }
}

fn junit_to_shallow_test_run(
    junit: Report,
    _root_path: &Path,
) -> Result<ShallowTestRun, anyhow::Error> {
    let report_name = junit.name.to_string();
    let name = if report_name == "nextest-run" {
        junit.uuid.map(|id| id.to_string()).unwrap_or(report_name)
    } else {
        report_name
    };
    let utc_date = junit.timestamp.and_then(|t| to_offset_datetime(t).ok());

    let mut inner_test_runs = Vec::with_capacity(junit.test_suites.len());
    for test_suite in junit.test_suites {
        inner_test_runs.push(get_inner_test_run(test_suite, &name, utc_date.clone())?);
    }

    Ok(ShallowTestRun {
        name,
        utc_date,
        revisions: None,
        origin: None,
        nr_of_test_cases: junit.tests.try_into()?,
        properties: None,
        duration: junit.time.and_then(|d| Duration::try_from(d).ok()),
        logs: None,
        test_cases: vec![],
        test_runs: inner_test_runs,
        description: None,
    })
}

pub struct WellKnownCoverageData {
    pub timestamp: Option<OffsetDateTime>,
    pub covered_files: Vec<CoveredFile>,
}

pub trait WellKnownCoverageConversion {
    fn to_well_known_coverage(
        &self,
        root_path: &Path,
        extension: &str,
        content: &str,
    ) -> Result<WellKnownCoverageData, anyhow::Error>;
}

impl WellKnownCoverageConversion for WellKnownCoverageFormat {
    fn to_well_known_coverage(
        &self,
        root_path: &Path,
        extension: &str,
        content: &str,
    ) -> Result<WellKnownCoverageData, anyhow::Error> {
        match self {
            WellKnownCoverageFormat::CoberturaLoose => {
                if extension == "xml" {
                    let cobertura =
                        match covcon::cobertura::schema_loose::Coverage::try_from(content) {
                            Ok(cov) => cov,
                            Err(err) => return Err(anyhow::Error::from_boxed(err)),
                        };
                    Ok(cobertura_to_well_known_coverage(
                        cobertura.into(),
                        root_path,
                    )?)
                } else if extension == "json" || extension == "json5" {
                    let cobertura =
                        json5::from_str::<covcon::cobertura::no_xml_loose::Coverage>(content)?;
                    Ok(cobertura_to_well_known_coverage(cobertura, root_path)?)
                } else {
                    anyhow::bail!(
                        "Got unsupported coverage format extension for Cobertura '{}'",
                        extension
                    )
                }
            }
        }
    }
}

fn cobertura_to_well_known_coverage(
    cobertura: covcon::cobertura::no_xml_loose::Coverage,
    root_path: &Path,
) -> Result<WellKnownCoverageData, anyhow::Error> {
    let timestamp = cobertura
        .timestamp
        .parse::<i64>()
        .ok()
        .and_then(|t| OffsetDateTime::from_unix_timestamp(t).ok());

    let mut covered_files = Vec::new();
    for package in cobertura.packages.package {
        for class in package.classes.class {
            match get_filepath(&class.filename, &root_path) {
                Ok(filepath) => {
                    covered_files.push(CoveredFile {
                        filepath,
                        file_hash: None,
                        lines: class
                            .lines
                            .iter()
                            .map(|l| CoveredLine {
                                nr: l.number.cast_signed(),
                                hits: l.hits.map(|h| h.cast_signed()),
                            })
                            .collect(),
                    });
                }
                Err(_) => {
                    eprintln!(
                        "Ignoring coverage data for file '{}'. Path could not be converted to a relative one.",
                        class.filename.display()
                    );
                }
            }
        }
    }

    Ok(WellKnownCoverageData {
        timestamp,
        covered_files,
    })
}

fn get_filepath(filename: &Path, root: &Path) -> Result<RelativePathBuf, anyhow::Error> {
    if filename.is_relative() {
        Ok(RelativePathBuf::from_path(filename)?)
    } else {
        Ok(filename.relative_to(root)?)
    }
}

fn get_inner_test_run(
    testsuite: TestSuite,
    name_prefix: &str,
    timestamp: Option<OffsetDateTime>,
) -> Result<ShallowTestRun, anyhow::Error> {
    let name = format!("{name_prefix}/{}", testsuite.name.as_str());

    let mut test_cases = Vec::with_capacity(testsuite.test_cases.len());
    for test_case in testsuite.test_cases {
        let test_case_name = test_case.name.as_str();
        let test_name = match test_case.classname {
            Some(class) => {
                if test_case_name.contains("::") {
                    // Likely Rust
                    format!("{}::{test_case_name}", class.as_str())
                } else {
                    format!("{}/{test_case_name}", class.as_str())
                }
            }
            None => test_case_name.to_string(),
        };
        let state = junit_status_to_mantra_state(test_case.status);

        test_cases.push(TestCase {
            name: test_name,
            properties: None,
            location: None,
            state,
            state_properties: None,
            logs: None,
            verified_reqs: vec![],
            covered_files: vec![],
            description: None,
            utc_date: test_case.timestamp.and_then(|t| to_offset_datetime(t).ok()),
            duration: test_case.time.and_then(|d| Duration::try_from(d).ok()),
        });
    }

    Ok(ShallowTestRun {
        name: name.to_string(),
        utc_date: testsuite
            .timestamp
            .and_then(|t| to_offset_datetime(t).ok())
            .or(timestamp),
        revisions: None,
        origin: None,
        nr_of_test_cases: testsuite.tests.try_into()?,
        properties: None,
        duration: testsuite.time.and_then(|d| Duration::try_from(d).ok()),
        logs: None,
        test_cases,
        test_runs: vec![],
        description: None,
    })
}

fn to_offset_datetime(
    timestamp: chrono::DateTime<FixedOffset>,
) -> Result<OffsetDateTime, anyhow::Error> {
    Ok(
        time::OffsetDateTime::from_unix_timestamp(timestamp.timestamp())?
            + Duration::nanoseconds(timestamp.timestamp_subsec_nanos().into()),
    )
}

fn junit_status_to_mantra_state(status: TestCaseStatus) -> TestCaseState {
    match status {
        TestCaseStatus::Success { flaky_runs: _ } => TestCaseState::Passed,
        TestCaseStatus::NonSuccess {
            kind: _,
            message: _,
            ty: _,
            description: _,
            reruns: _,
        } => TestCaseState::Failed,
        TestCaseStatus::Skipped {
            message: _,
            ty: _,
            description: _,
        } => TestCaseState::Skipped,
    }
}
