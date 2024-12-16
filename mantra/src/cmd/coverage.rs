use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use mantra_lang_tracing::path::SlashPathBuf;
use mantra_schema::{
    coverage::{CoverageSchema, CoveredFileTrace, CoveredLine, TestRunPk},
    requirements::ReqId,
    Line,
};
use time::OffsetDateTime;

use crate::db::{DbError, MantraDb, TracePk};

pub struct CoverageChanges {
    inserted: Vec<TracePk>,
}

impl std::fmt::Display for CoverageChanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.inserted.is_empty() {
            writeln!(f, "No coverage information was added.")
        } else {
            writeln!(f, "Coverage added for traces:")?;
            for covered_trace in &self.inserted {
                writeln!(f, "- {covered_trace}")?;
            }

            Ok(())
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// Files containing coverage data according to the *mantra* CoverageSchema.
    /// The file format may either be JSON or TOML.
    #[serde(
        alias = "filepaths",
        alias = "external-files",
        alias = "external-filepaths"
    )]
    pub files: Vec<PathBuf>,
}

pub fn iso8601_str_to_offsetdatetime(time_str: &str) -> OffsetDateTime {
    OffsetDateTime::parse(
        time_str,
        &time::format_description::well_known::Iso8601::DEFAULT,
    )
    .expect("Test run date was added to db in ISO8601 format.")
}

#[derive(Debug, thiserror::Error)]
pub enum CoverageError {
    #[error("{}", .0)]
    ReadingData(String),
    #[error("{}", .0)]
    Deserialize(serde_json::Error),
    #[error("{}", .0)]
    Db(DbError),
}

pub async fn collect_from_path(
    db: &MantraDb,
    data_file: &Path,
) -> Result<CoverageChanges, CoverageError> {
    let data = std::fs::read_to_string(data_file).map_err(|_| {
        CoverageError::ReadingData(format!(
            "Could not read coverage data from '{}'.",
            data_file.display()
        ))
    })?;

    collect_from_str(db, &data).await
}

pub async fn collect_from_str(db: &MantraDb, data: &str) -> Result<CoverageChanges, CoverageError> {
    let coverage =
        serde_json::from_str::<CoverageSchema>(data).map_err(CoverageError::Deserialize)?;

    let mut changes = CoverageChanges {
        inserted: Vec::new(),
    };

    for test_run in coverage.test_runs {
        if db.test_run_exists(&test_run.name, &test_run.date).await {
            log::info!(
                "Skipping test run '{}' at {}, because it already exists in the database.",
                &test_run.name,
                &test_run.date,
            );
            continue;
        }

        db.add_test_run(
            &test_run.name,
            &test_run.date,
            test_run.nr_of_tests,
            test_run.meta,
            test_run.logs,
        )
        .await
        .map_err(CoverageError::Db)?;

        let test_run_pk = TestRunPk {
            name: test_run.name,
            date: test_run.date,
        };

        for test in test_run.tests {
            db.add_test(
                &test_run_pk,
                &test.name,
                &test.filepath,
                test.line,
                test.state,
            )
            .await
            .map_err(CoverageError::Db)?;

            for mut file in test.covered_files {
                if let Ok(Some(mut traces)) =
                    covered_lines_to_traces(db, file.filepath.clone(), &mut file.covered_lines)
                        .await
                {
                    file.covered_traces.append(&mut traces);
                }

                for trace in file.covered_traces {
                    for req_id in trace.req_ids {
                        let db_result = db
                            .add_coverage(
                                &test_run_pk,
                                &test.name,
                                &file.filepath,
                                trace.line,
                                &req_id,
                            )
                            .await;

                        match db_result {
                            Ok(true) => {
                                changes.inserted.push(TracePk {
                                    req_id,
                                    filepath: file.filepath.clone(),
                                    line: trace.line,
                                });
                            }
                            Ok(false) => {
                                log::info!(
                                "Found unrelated coverage for reg-id=`{}`, file='{}', line='{}'.",
                                req_id,
                                file.filepath.display(),
                                trace.line
                            );
                            }
                            Err(_) => {
                                db_result.map_err(CoverageError::Db)?;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(changes)
}

async fn covered_lines_to_traces(
    db: &MantraDb,
    filepath: PathBuf,
    covered_lines: &mut [CoveredLine],
) -> Result<Option<Vec<CoveredFileTrace>>, DbError> {
    let mut traces = Vec::new();

    let file = SlashPathBuf::from(filepath);
    let file_str = file.to_string();

    let trace_spans = sqlx::query!(
        "select req_id, filepath, line, start, end from TraceSpans where filepath = $1",
        file_str,
    )
    .fetch_all(db.pool())
    .await
    .map_err(|err| DbError::Query(err.to_string()))?
    .into_iter()
    .map(|record| intervaltree::Element {
        range: (record.start as Line)..(record.end as Line),
        value: (record.req_id, record.line as Line),
    })
    .collect();

    traces.extend(get_covered_traces(trace_spans, covered_lines));

    if traces.is_empty() {
        Ok(None)
    } else {
        Ok(Some(traces))
    }
}

fn get_covered_traces(
    trace_spans: Vec<intervaltree::Element<Line, (String, Line)>>,
    covered_lines: &mut [CoveredLine],
) -> impl Iterator<Item = CoveredFileTrace> {
    let mut traces: HashMap<Line, HashSet<ReqId>> = HashMap::new();
    let tree = intervaltree::IntervalTree::from_iter(trace_spans);

    covered_lines.sort();

    for covered_line in covered_lines {
        for interval in tree.query_point(covered_line.line) {
            traces
                .entry(interval.value.1)
                .or_default()
                .insert(interval.value.0.clone());
        }
    }

    traces.into_iter().map(|(line, req_ids)| CoveredFileTrace {
        req_ids: req_ids.into_iter().collect(),
        line,
    })
}

#[cfg(test)]
mod test {
    use intervaltree::Element;
    use mantra_schema::coverage::{CoveredFileTrace, CoveredLine};

    use super::get_covered_traces;

    #[test]
    fn disjoint_traces() {
        let spans = vec![
            Element {
                range: 10..15,
                value: ("first".to_string(), 8),
            },
            Element {
                range: 20..25,
                value: ("second".to_string(), 18),
            },
        ];

        // range for first trace is 10..15, so 15 is exclusive
        let mut lines = vec![
            CoveredLine { line: 15, hits: 0 },
            CoveredLine { line: 24, hits: 0 },
            CoveredLine { line: 30, hits: 0 },
        ];

        let covered_traces: Vec<CoveredFileTrace> = get_covered_traces(spans, &mut lines).collect();

        assert_eq!(
            covered_traces.len(),
            1,
            "Not just the second trace matched."
        );
        assert_eq!(
            covered_traces.first().unwrap().req_ids.first().unwrap(),
            "second",
            "The second trace was not matched."
        );
    }

    #[test]
    fn two_overlapping_traces() {
        let spans = vec![
            Element {
                range: 10..25,
                value: ("outer".to_string(), 8),
            },
            Element {
                range: 20..24,
                value: ("inner".to_string(), 18),
            },
        ];

        let mut lines = vec![CoveredLine { line: 20, hits: 0 }];

        let covered_traces: Vec<CoveredFileTrace> = get_covered_traces(spans, &mut lines).collect();

        assert_eq!(covered_traces.len(), 2, "Both traces matched.");
        assert_ne!(
            covered_traces.first().unwrap().req_ids.first().unwrap(),
            covered_traces.last().unwrap().req_ids.first().unwrap(),
            "The same trace was matched twice."
        );
    }
}
