use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use mantra_lang_tracing::path::SlashPathBuf;
use mantra_schema::{
    coverage::{CoverageSchema, LineCoverage, TestRunPk},
    traces::TracePk,
    Line,
};
use time::OffsetDateTime;

use crate::db::{DbError, MantraDb};

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

#[derive(Debug, Clone, clap::Args, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// Files containing coverage data according to the *mantra* CoverageSchema.
    /// The file format may either be JSON or TOML.
    #[serde(alias = "data-paths")]
    pub data: Vec<PathBuf>,
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

        for mut test in test_run.tests {
            db.add_test(
                &test_run_pk,
                &test.name,
                &test.filepath,
                test.line,
                test.state,
            )
            .await
            .map_err(CoverageError::Db)?;

            if let Ok(Some(mut traces)) = covered_lines_to_traces(db, &mut test.covered_lines).await
            {
                test.covered_traces.append(&mut traces);
            }

            for trace in test.covered_traces {
                let db_result = db
                    .add_coverage(
                        &test_run_pk,
                        &test.name,
                        &trace.filepath,
                        trace.line,
                        &trace.req_id,
                    )
                    .await;

                match db_result {
                    Ok(true) => {
                        changes.inserted.push(trace);
                    }
                    Ok(false) => {
                        log::info!(
                            "Found unrelated coverage for reg-id=`{}`, file='{}', line='{}'.",
                            trace.req_id,
                            trace.filepath.display(),
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

    Ok(changes)
}

async fn covered_lines_to_traces(
    db: &MantraDb,
    covered_lines: &mut [LineCoverage],
) -> Result<Option<Vec<TracePk>>, DbError> {
    let mut traces = Vec::new();

    for coverage in covered_lines {
        let file = SlashPathBuf::from(coverage.filepath.clone());
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
            value: TracePk {
                req_id: record.req_id,
                filepath: file.clone(),
                line: record.line as Line,
            },
        })
        .collect();

        traces.extend(get_covered_traces(trace_spans, &mut coverage.lines));
    }

    if traces.is_empty() {
        Ok(None)
    } else {
        Ok(Some(traces))
    }
}

fn get_covered_traces(
    trace_spans: Vec<intervaltree::Element<Line, TracePk>>,
    covered_lines: &mut [Line],
) -> impl Iterator<Item = TracePk> {
    let mut traces = HashSet::new();
    let tree = intervaltree::IntervalTree::from_iter(trace_spans);

    covered_lines.sort();

    for covered_line in covered_lines {
        for interval in tree.query_point(*covered_line) {
            traces.insert(interval.value.clone());
        }
    }

    traces.into_iter()
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use intervaltree::Element;
    use mantra_schema::traces::TracePk;

    use super::get_covered_traces;

    #[test]
    fn disjoint_traces() {
        let spans = vec![
            Element {
                range: 10..15,
                value: TracePk {
                    req_id: "first".to_string(),
                    filepath: PathBuf::from("filepath.rs"),
                    line: 8,
                },
            },
            Element {
                range: 20..25,
                value: TracePk {
                    req_id: "second".to_string(),
                    filepath: PathBuf::from("filepath.rs"),
                    line: 18,
                },
            },
        ];

        // range for first trace is 10..15, so 15 is exclusive
        let mut lines = vec![15, 24, 30];

        let covered_traces: Vec<TracePk> = get_covered_traces(spans, &mut lines).collect();

        assert_eq!(
            covered_traces.len(),
            1,
            "Not just the second trace matched."
        );
        assert_eq!(
            covered_traces.first().unwrap().req_id,
            "second",
            "The second trace was not matched."
        );
    }

    #[test]
    fn two_overlapping_traces() {
        let spans = vec![
            Element {
                range: 10..25,
                value: TracePk {
                    req_id: "outer".to_string(),
                    filepath: PathBuf::from("filepath.rs"),
                    line: 8,
                },
            },
            Element {
                range: 20..24,
                value: TracePk {
                    req_id: "inner".to_string(),
                    filepath: PathBuf::from("filepath.rs"),
                    line: 18,
                },
            },
        ];

        let mut lines = vec![20];

        let covered_traces: Vec<TracePk> = get_covered_traces(spans, &mut lines).collect();

        assert_eq!(covered_traces.len(), 2, "Both traces matched.");
        assert_ne!(
            covered_traces.first().unwrap().req_id,
            covered_traces.last().unwrap().req_id,
            "The same trace was matched twice."
        );
    }
}
