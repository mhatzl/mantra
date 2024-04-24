use std::path::{Path, PathBuf};

use defmt_json_schema::{v1::JsonFrame, SchemaVersion};
use regex::Regex;

use crate::db::{DbError, MantraDb};

#[derive(Debug, clap::Args)]
pub struct CliConfig {
    /// Data containing coverage logs to retrieve coverage information.
    pub data_file: PathBuf,
    #[command(flatten)]
    pub cfg: Config,
}

#[derive(Debug, clap::Args)]
pub struct Config {
    pub project_name: String,
    pub root: PathBuf,
    /// Optional prefix set before identifiers of test functions.
    pub test_prefix: Option<String>,
    #[arg(value_enum)]
    pub fmt: LogFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum LogFormat {
    DefmtJson,
}

#[derive(Debug, thiserror::Error)]
pub enum CoverageError {
    #[error("Failed to read coverage data. Cause: {}", .0)]
    ReadingData(String),
    #[error("Failed to extract coverage data from defmt logs. Cause: {}", .0)]
    DefmtJson(String),
    #[error("Database error while updating coverage data. Cause: {}", .0)]
    Db(DbError),
}

pub async fn coverage_from_path(
    data: &Path,
    db: &MantraDb,
    cfg: &Config,
) -> Result<(), CoverageError> {
    let data_str = std::fs::read_to_string(data).map_err(|err| {
        CoverageError::ReadingData(format!(
            "Could not read coverage data from '{}'. Cause: {}",
            data.display(),
            err
        ))
    })?;

    coverage_from_str(&data_str, db, cfg).await
}

pub async fn coverage_from_str(
    data: &str,
    db: &MantraDb,
    cfg: &Config,
) -> Result<(), CoverageError> {
    match cfg.fmt {
        LogFormat::DefmtJson => {
            coverage_from_defmtjson(
                data,
                db,
                &cfg.project_name,
                &cfg.root,
                cfg.test_prefix.as_deref(),
            )
            .await
        }
    }
}

pub async fn coverage_from_defmt_frames(
    frames: &[JsonFrame],
    db: &MantraDb,
    project_name: &str,
    root: &Path,
    test_prefix: Option<&str>,
) -> Result<(), CoverageError> {
    let mut current_test_fn = None;

    for frame in frames {
        current_test_fn =
            add_frame_to_db(frame, db, project_name, root, test_prefix, current_test_fn).await?;
    }

    Ok(())
}

async fn add_frame_to_db(
    frame: &JsonFrame,
    db: &MantraDb,
    project_name: &str,
    root: &Path,
    test_prefix: Option<&str>,
    mut current_test_fn: Option<String>,
) -> Result<Option<String>, CoverageError> {
    let test_fn_matcher = TEST_FN_MATCHER.get_or_init(|| {
        Regex::new(r"^\(\d+/\d+\)\s(?<state>(?:running)|(?:ignoring))\s`(?<fn_name>.+)`...")
            .expect("Could not create regex matcher for defmt test-fn entries.")
    });

    if let Some(captured_test_fn) = test_fn_matcher.captures(&frame.data) {
        let fn_state = captured_test_fn
            .name("state")
            .expect("State of the test-fn was not captured.");
        let fn_name = captured_test_fn
            .name("fn_name")
            .expect("Name of the test-fn was not captured.");

        let Some(file) = &frame.location.file else {
            return Err(CoverageError::DefmtJson(format!(
                "Missing file location information for log entry '{}'.",
                frame.data
            )));
        };
        let Some(line_nr) = frame.location.line else {
            return Err(CoverageError::DefmtJson(format!(
                "Missing line location information for log entry '{}'.",
                frame.data
            )));
        };
        let Some(mod_path) = &frame.location.module_path else {
            return Err(CoverageError::DefmtJson(format!(
                "Missing line location information for log entry '{}'.",
                frame.data
            )));
        };
        let mod_path_str = format!(
            "{}{}",
            mod_path.crate_name,
            if mod_path.modules.is_empty() {
                String::new()
            } else {
                format!("::{}", mod_path.modules.join("::"))
            }
        );

        match fn_state.as_str() {
            "running" => {
                let test_fn_name = if let Some(prefix) = test_prefix {
                    format!("{}:{}::{}", prefix, mod_path_str, fn_name.as_str())
                } else {
                    format!("{}::{}", mod_path_str, fn_name.as_str())
                };
                current_test_fn = Some(test_fn_name.clone());

                db.add_test(
                    &test_fn_name,
                    project_name,
                    root,
                    &PathBuf::from(file),
                    line_nr,
                )
                .await.map_err(CoverageError::Db)?;
            }
            "ignoring" => {
                current_test_fn = None;
            }
            _ => unreachable!("Invalid state '{}' for test function '{}' in log entry '{}'. Only 'running' and 'ignoring' are allowed.", fn_state.as_str(), fn_name.as_str(), frame.data),
        }
    } else if let Some(covered_req) =
        mantra_rust_macros::extract::extract_first_coverage(&frame.data)
    {
        // mantra logs may be set outside test runs => those cannot be added as test coverage
        if let Some(ref current_test) = current_test_fn {
            db.add_coverage(
                project_name,
                root,
                current_test,
                &covered_req.file,
                covered_req.line,
                &covered_req.id,
            )
            .await
            .map_err(CoverageError::Db)?;
        };
    }

    Ok(current_test_fn)
}

async fn coverage_from_defmtjson(
    data: &str,
    db: &MantraDb,
    project_name: &str,
    root: &Path,
    test_prefix: Option<&str>,
) -> Result<(), CoverageError> {
    let lines = data.lines().collect::<Vec<_>>();

    let schema_version: SchemaVersion =
        serde_json::from_str(lines.first().ok_or(CoverageError::DefmtJson(
            "Missing defmt schema version at start of given data.".to_string(),
        ))?)
        .map_err(|err| {
            CoverageError::DefmtJson(format!(
                "Could not extract defmt schema version from given data. Cause: {}",
                err
            ))
        })?;

    match schema_version {
        defmt_json_schema::v1::SCHEMA_VERSION => {
            let mut current_test_fn = None;

            for line in &lines[1..] {
                if line.is_empty() {
                    continue;
                }

                let frame: JsonFrame = serde_json::from_str(line).map_err(|err| {
                    CoverageError::DefmtJson(format!(
                        "Could not extract defmt log frame from line '{}'. Cause: {}",
                        line, err
                    ))
                })?;

                current_test_fn =
                    add_frame_to_db(&frame, db, project_name, root, test_prefix, current_test_fn)
                        .await?;
            }
        }
        _ => {
            return Err(CoverageError::DefmtJson(
                "Only defmt schema version 1 is supported for now.".to_string(),
            ))
        }
    }

    Ok(())
}

static TEST_FN_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
