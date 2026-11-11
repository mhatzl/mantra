use std::path::Path;

use anyhow::Context;
use chrono::FixedOffset;
use mantra_schema::{
    Origin, Properties, Revision,
    jiff::{SignedDuration, Timestamp, Zoned, tz::TimeZone},
    path::{PathExt, RelativePathBuf},
    test_runs::{CoveredFile, CoveredLine, LogOutput, TestCase, TestCaseState, TestRun},
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
    pub utc_date: Option<Zoned>,
    /// Optional description of the test run.
    pub description: Option<String>,
    /// Optional revisions for the test run.
    pub revisions: Option<Vec<Revision>>,
    /// Optional origin of the test run.
    /// [req("testcov.test_run.origin")]
    pub origin: Option<Origin>,
    /// Optional field to store custom information per test run.
    /// [req("testcov.test_run.metadata")]
    pub properties: Option<Properties>,
    /// Optional duration about how long the test run took.
    /// Will be displayed in seconds with nanosecond precision in decimal form.
    pub duration_sec: Option<SignedDuration>,
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
    pub fn into_test_run(
        self,
        covered_files: Vec<CoveredFile>,
        coverage_timestamp: Option<&Zoned>,
    ) -> TestRun {
        let utc_date = match self.utc_date.or(coverage_timestamp.cloned()) {
            Some(timestamp) => timestamp,
            None => {
                log::warn!(
                    "No timestamp collected for test run '{}'. Using local timestamp.",
                    self.name
                );
                Zoned::now()
            }
        };

        let test_runs = self
            .test_runs
            .into_iter()
            .map(|t| t.into_test_run(vec![], Some(&utc_date)))
            .collect();

        TestRun {
            name: self.name,
            utc_date,
            description: self.description,
            revisions: self.revisions,
            origin: self.origin,
            properties: self.properties,
            duration_sec: self.duration_sec,
            logs: self.logs,
            test_cases: self.test_cases,
            covered_files,
            test_runs,
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
                    let junit_report = quick_junit::Report::deserialize_from_str(content)
                        .context("Failed reading JUnit content")?;
                    Ok(junit_to_shallow_test_run(junit_report, root_path)
                        .context("Failed converting JUnit to mantra's test run schema")?)
                } else {
                    anyhow::bail!(
                        "Got unsupported test format extension '{}' for JUnit ",
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
        inner_test_runs.push(
            get_inner_test_run(test_suite, &name, utc_date.as_ref())
                .context("Failed to convert inner test runs")?,
        );
    }

    Ok(ShallowTestRun {
        name,
        utc_date,
        revisions: None,
        origin: None,
        properties: None,
        duration_sec: junit.time.and_then(|d| SignedDuration::try_from(d).ok()),
        logs: None,
        test_cases: vec![],
        test_runs: inner_test_runs,
        description: None,
    })
}

pub struct WellKnownCoverageData {
    pub timestamp: Option<Zoned>,
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
                            Err(err) => {
                                return Err(anyhow::Error::from_boxed(err)
                                    .context("Failed deserializing Cobertura XML content"));
                            }
                        };
                    Ok(cobertura_to_well_known_coverage(
                        cobertura.into(),
                        root_path,
                    )?)
                } else if extension == "json" || extension == "json5" {
                    let cobertura =
                        json5::from_str::<covcon::cobertura::no_xml_loose::Coverage>(content)
                            .context("Failed deserializing Cobertura JSON content")?;
                    Ok(cobertura_to_well_known_coverage(cobertura, root_path)?)
                } else {
                    anyhow::bail!(
                        "Got unsupported coverage format extension '{}' for Cobertura",
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
    let raw_timestamp = cobertura.timestamp.parse::<i64>().ok();

    let timestamp = raw_timestamp.and_then(|rt| {
        // Cobertura has no fixed unit and different tools set milliseconds or seconds, so we do some heurstics check.
        // 10 digits ≈ seconds, 13 ≈ milliseconds
        let timestamp = if rt > 99_999_999_999 {
            Timestamp::from_millisecond(rt)
        } else {
            Timestamp::from_second(rt)
        };

        timestamp.map(|t| Zoned::new(t, TimeZone::system())).ok()
    });

    let mut covered_files = Vec::new();
    for package in cobertura.packages.package {
        for class in package.classes.class {
            match get_filepath(&class.filename, root_path) {
                Ok(filepath) => {
                    covered_files.push(CoveredFile {
                        filepath,
                        file_hash: None,
                        lines: class
                            .lines
                            .iter()
                            .map(|l| CoveredLine {
                                number: l.number.cast_signed(),
                                hits: l.hits.map(|h| h.cast_signed()),
                            })
                            .collect(),
                    });
                }
                Err(_) => {
                    log::warn!(
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
    timestamp: Option<&Zoned>,
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
            duration_sec: test_case
                .time
                .and_then(|d| SignedDuration::try_from(d).ok()),
        });
    }

    Ok(ShallowTestRun {
        name: name.to_string(),
        utc_date: testsuite
            .timestamp
            .and_then(|t| to_offset_datetime(t).ok())
            .or(timestamp.cloned()),
        revisions: None,
        origin: None,
        properties: None,
        duration_sec: testsuite
            .time
            .and_then(|d| SignedDuration::try_from(d).ok()),
        logs: None,
        test_cases,
        test_runs: vec![],
        description: None,
    })
}

fn to_offset_datetime(datetime: chrono::DateTime<FixedOffset>) -> Result<Zoned, anyhow::Error> {
    let timestamp = Timestamp::from_nanosecond(
        i128::from(datetime.timestamp()) * 1_000_000_000
            + i128::from(datetime.timestamp_subsec_nanos()),
    )
    .context("Timestamp outside valid range for the `jiff` crate")?;
    Ok(timestamp.to_zoned(TimeZone::system()))
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
