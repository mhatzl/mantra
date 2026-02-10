use std::path::{Path, PathBuf};

use chrono::FixedOffset;
use mantra_schema::{
    path::{PathExt, RelativePathBuf},
    test_runs::{CoveredFile, CoveredLine, TestCase, TestCaseState, TestRun},
    time::{self, Duration, OffsetDateTime},
};
use quick_junit::{TestCaseStatus, TestSuite};

pub fn test_run_from_paths(
    junit_path: &Path,
    cobertura_path: &Path,
    language: ProgramLanguage,
) -> Result<TestRun, anyhow::Error> {
    let junit_content = std::fs::read_to_string(junit_path)?;
    let junit = quick_junit::Report::deserialize_from_str(&junit_content)?;

    let cobertura_content = std::fs::read_to_string(cobertura_path)?;
    let cobertura =
        match covcon::cobertura::schema_loose::Coverage::try_from(cobertura_content.as_str()) {
            Ok(cov) => cov,
            Err(err) => return Err(anyhow::Error::from_boxed(err)),
        };

    to_test_run(junit, cobertura, language)
}

pub fn to_test_run(
    junit: quick_junit::Report,
    cobertura: covcon::cobertura::schema_loose::Coverage,
    language: ProgramLanguage,
) -> Result<TestRun, anyhow::Error> {
    let report_name = junit.name.to_string();
    let name = if report_name == "nextest-run" {
        junit.uuid.map(|id| id.to_string()).unwrap_or(report_name)
    } else {
        report_name
    };
    let utc_date = match junit.timestamp.and_then(|t| {
        Some(
            time::OffsetDateTime::from_unix_timestamp(t.timestamp()).ok()?
                + Duration::nanoseconds(t.timestamp_subsec_nanos().into()),
        )
    }) {
        Some(utc_date) => utc_date,
        None => {
            // "No timestamp set on JUnit or Cobertura input that could be converted to a UTC DateTime."
            OffsetDateTime::from_unix_timestamp(cobertura.timestamp.parse()?)?
        }
    };

    let root_path = get_root();

    let mut covered_files = Vec::new();
    for package in cobertura.packages.package {
        match language {
            ProgramLanguage::Auto => todo!(),
            // Note: cargo-nextest seems to duplicate the info for package and class => skip package entirely
            ProgramLanguage::Rust => {
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
                                "Coverage date for files outside the working directory are ignored! Path: '{}'",
                                class.filename.display()
                            );
                        }
                    }
                }
            }
        }
    }

    let mut inner_test_runs = Vec::with_capacity(junit.test_suites.len());
    for test_suite in junit.test_suites {
        inner_test_runs.push(get_rust_sub_test_run(test_suite, &name, &utc_date)?);
    }

    let main_test_run = TestRun {
        name: name.to_string(),
        utc_date,
        revisions: None,
        origin: None,
        nr_of_test_cases: junit.tests.try_into()?,
        properties: None,
        duration: junit.time.and_then(|d| Duration::try_from(d).ok()),
        logs: None,
        test_cases: vec![],
        covered_files,
        test_runs: inner_test_runs,
        description: None,
    };

    Ok(main_test_run)
}

pub enum ProgramLanguage {
    Auto,
    Rust,
}

fn get_root() -> PathBuf {
    PathBuf::from("/Users/manuelhatzl/Documents/Projects/mantra/")
}

fn get_filepath(filename: &Path, root: &Path) -> Result<RelativePathBuf, anyhow::Error> {
    if filename.is_relative() {
        Ok(RelativePathBuf::from_path(filename)?)
    } else {
        match filename.relative_to(root) {
            Ok(filepath) => Ok(filepath),
            Err(_) => {
                anyhow::bail!("Coverage date for files outside the working directory are ignored!");
            }
        }
    }
}

fn get_rust_sub_test_run(
    testsuite: TestSuite,
    name_prefix: &str,
    timestamp: &OffsetDateTime,
) -> Result<TestRun, anyhow::Error> {
    let name = format!("{name_prefix}/{}", testsuite.name.as_str());

    let mut test_cases = Vec::with_capacity(testsuite.test_cases.len());
    for test_case in testsuite.test_cases {
        let test_case_name = test_case.name.as_str();
        let test_name = match test_case.classname {
            Some(class) => format!("{}::{test_case_name}", class.as_str()),
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

    Ok(TestRun {
        name: name.to_string(),
        utc_date: testsuite
            .timestamp
            .and_then(|t| to_offset_datetime(t).ok())
            .unwrap_or(*timestamp),
        revisions: None,
        origin: None,
        nr_of_test_cases: testsuite.tests.try_into()?,
        properties: None,
        duration: testsuite.time.and_then(|d| Duration::try_from(d).ok()),
        logs: None,
        test_cases,
        covered_files: vec![],
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
