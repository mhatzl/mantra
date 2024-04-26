use std::path::{Path, PathBuf};

use defmt_json_schema::v1::JsonFrame;
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
            let mut frames = Vec::new();
            for line in data.lines() {
                frames.push(serde_json::from_str::<JsonFrame>(line).map_err(|err| {
                    CoverageError::DefmtJson(format!(
                        "Could not extract defmt log frame from line '{}'. Cause: {}",
                        line, err
                    ))
                })?);
            }

            coverage_from_defmt_frames(&frames, db, &cfg.project_name, cfg.test_prefix.as_deref())
                .await
        }
    }
}

pub async fn coverage_from_defmt_frames(
    frames: &[JsonFrame],
    db: &MantraDb,
    project_name: &str,
    test_prefix: Option<&str>,
) -> Result<(), CoverageError> {
    let mut current_test_fn: Option<String> = None;

    for frame in frames {
        let new_test_fn = add_frame_to_db(
            frame,
            db,
            project_name,
            test_prefix,
            current_test_fn.as_deref(),
        )
        .await?;

        if current_test_fn != new_test_fn {
            if let Some(passed_test_fn) = &current_test_fn {
                db.test_passed(passed_test_fn, project_name)
                    .await
                    .map_err(CoverageError::Db)?;
            }
        }

        current_test_fn = new_test_fn;
    }

    Ok(())
}

async fn add_frame_to_db(
    frame: &JsonFrame,
    db: &MantraDb,
    project_name: &str,
    test_prefix: Option<&str>,
    current_test_fn: Option<&str>,
) -> Result<Option<String>, CoverageError> {
    let test_fn_matcher = TEST_FN_MATCHER.get_or_init(|| {
        Regex::new(r"^\(\d+/\d+\)\s(?<state>(?:running)|(?:ignoring))\s`(?<fn_name>.+)`...")
            .expect("Could not create regex matcher for defmt test-fn entries.")
    });
    let mut new_test_fn: Option<String> = current_test_fn.map(|t| t.to_string());

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
                new_test_fn = Some(test_fn_name.clone());

                db.add_test(
                    &test_fn_name,
                    project_name,
                    &PathBuf::from(file),
                    line_nr,
                None, // test result is always unknown at the start
                )
                .await.map_err(CoverageError::Db)?;
            }
            "ignoring" => {
                new_test_fn = None;
            }
            _ => unreachable!("Invalid state '{}' for test function '{}' in log entry '{}'. Only 'running' and 'ignoring' are allowed.", fn_state.as_str(), fn_name.as_str(), frame.data),
        }
    } else if let Some(covered_req) =
        mantra_rust_macros::extract::extract_first_coverage(&frame.data)
    {
        // mantra logs may be set outside test runs => those cannot be added as test coverage
        if let Some(current_test) = current_test_fn {
            let db_result = db
                .add_coverage(
                    project_name,
                    current_test,
                    &covered_req.file,
                    covered_req.line,
                    &covered_req.id,
                )
                .await;

            // mantra logs might be set in external crates, but the matching traces are likely missing
            if let Err(DbError::ForeignKeyViolation(_)) = &db_result {
                log::debug!("Foreign key violation while inserting coverage for reg-id=`{}`, file='{}', line='{}'.", covered_req.id, covered_req.file.display(), covered_req.line);
            } else {
                db_result.map_err(CoverageError::Db)?;
            }
        };
    } else if frame.data == "all tests passed!" {
        if let Some(passed_test) = current_test_fn {
            db.test_passed(passed_test, project_name)
                .await
                .map_err(CoverageError::Db)?;
        }
    }

    Ok(new_test_fn)
}

// async fn coverage_from_defmtjson(
//     data: &str,
//     db: &MantraDb,
//     project_name: &str,
//     test_prefix: Option<&str>,
// ) -> Result<(), CoverageError> {
//     let lines = data.lines().collect::<Vec<_>>();

//     let schema_version: SchemaVersion =
//         serde_json::from_str(lines.first().ok_or(CoverageError::DefmtJson(
//             "Missing defmt schema version at start of given data.".to_string(),
//         ))?)
//         .map_err(|err| {
//             CoverageError::DefmtJson(format!(
//                 "Could not extract defmt schema version from given data. Cause: {}",
//                 err
//             ))
//         })?;

//     match schema_version {
//         defmt_json_schema::v1::SCHEMA_VERSION => {
//             let mut current_test_fn = None;

//             for line in &lines[1..] {
//                 if line.is_empty() {
//                     continue;
//                 }

//                 let frame: JsonFrame = serde_json::from_str(line).map_err(|err| {
//                     CoverageError::DefmtJson(format!(
//                         "Could not extract defmt log frame from line '{}'. Cause: {}",
//                         line, err
//                     ))
//                 })?;

//                 current_test_fn =
//                     add_frame_to_db(&frame, db, project_name, test_prefix, current_test_fn).await?;
//             }
//         }
//         _ => {
//             return Err(CoverageError::DefmtJson(
//                 "Only defmt schema version 1 is supported for now.".to_string(),
//             ))
//         }
//     }

//     Ok(())
// }

static TEST_FN_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
