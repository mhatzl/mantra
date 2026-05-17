use std::collections::HashMap;

use mantra_schema::{
    FmtHash, Line, SCHEMA_VERSION,
    annotations::TraceKind,
    path::RelativePathBuf,
    report::{
        annotations::TraceReference,
        product::ProductReportSchema,
        test_run::TestRunReference,
        test_runs::{TestRunsOverview, TestRunsReportSchema},
        tests::{
            CoverageSummary, ResolvedLineCoverageState, TestCoverage, TestCoverageSummary,
            TestCoveredFile, TestState, TestsSummary,
        },
    },
};

use crate::db::MantraTransaction;

pub(super) struct ResolvedLineCoverage {
    pub(super) filepath: String,
    pub(super) line: Line,
    pub(super) state: ResolvedLineCoverageState,
}

pub async fn generate_test_runs_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
) -> Result<TestRunsReportSchema, anyhow::Error> {
    let test_run_records = sqlx::query!(
        r#"
        select test_run_name, test_run_date, state as "state!:i64"
        from TestRunStates
        where product_id = $1
        "#,
        product.id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut test_runs_summary = TestsSummary {
        total: test_run_records.len() as i64,
        ..Default::default()
    };

    let mut passed = Vec::new();
    let mut failed = Vec::new();
    let mut skipped = Vec::new();
    let mut unknown = Vec::new();
    let mut obsolete = Vec::new();

    for test_run in test_run_records {
        let test_run_reference = TestRunReference {
            product_id: product.id.clone(),
            name: test_run.test_run_name,
            utc_date: mantra_schema::test_runs::test_date_from_str(&test_run.test_run_date)?,
            state: test_run.state.try_into()?,
        };

        match test_run_reference.state {
            mantra_schema::report::tests::TestState::Failed => failed.push(test_run_reference),
            mantra_schema::report::tests::TestState::Passed => passed.push(test_run_reference),
            mantra_schema::report::tests::TestState::Skipped => skipped.push(test_run_reference),
            mantra_schema::report::tests::TestState::Unknown => unknown.push(test_run_reference),
            mantra_schema::report::tests::TestState::Obsolete => obsolete.push(test_run_reference),
        }
    }

    test_runs_summary.failed.cnt = failed.len() as i64;
    test_runs_summary.passed.cnt = passed.len() as i64;
    test_runs_summary.skipped.cnt = skipped.len() as i64;
    test_runs_summary.unknown.cnt = unknown.len() as i64;
    test_runs_summary.obsolete.cnt = obsolete.len() as i64;

    test_runs_summary.update_percentages();

    let test_cases_states = sqlx::query!(
        r#"
        select state, count(test_run_name) as "cnt!:i64"
        from ResolvedTestCaseStates
        where product_id = $1
        group by state
        "#,
        product.id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut test_cases_summary = TestsSummary::default();

    for state_record in test_cases_states {
        let state = TestState::try_from(state_record.state)?;

        match state {
            TestState::Failed => test_cases_summary.failed.cnt += state_record.cnt,
            TestState::Passed => test_cases_summary.passed.cnt += state_record.cnt,
            TestState::Skipped => test_cases_summary.skipped.cnt += state_record.cnt,
            TestState::Unknown => test_cases_summary.unknown.cnt += state_record.cnt,
            TestState::Obsolete => test_cases_summary.obsolete.cnt += state_record.cnt,
        }
    }

    test_cases_summary.total = test_cases_summary.failed.cnt
        + test_cases_summary.passed.cnt
        + test_cases_summary.skipped.cnt
        + test_cases_summary.unknown.cnt
        + test_cases_summary.obsolete.cnt;

    test_cases_summary.update_percentages();

    let coverage = generate_overall_test_coverage(transaction, product).await?;

    Ok(TestRunsReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        test_cases_summary,
        test_runs: TestRunsOverview {
            summary: test_runs_summary,
            passed,
            failed,
            skipped,
            unknown,
            obsolete,
        },
        coverage,
    })
}

async fn generate_overall_test_coverage<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
) -> Result<TestCoverage, anyhow::Error> {
    let coverable_lines_record = sqlx::query!(
        r#"
        select sum(coverable_lines) as "coverable_lines!:i64"
        from CoverableLinesPerFilepath
        where product_id = $1
        "#,
        product.id
    )
    .fetch_one(transaction.as_mut())
    .await?;

    let mut test_summary = TestCoverageSummary {
        lines: CoverageSummary {
            total: coverable_lines_record.coverable_lines,
            ..Default::default()
        },
    };

    let resolved_lines: Vec<_> = sqlx::query!(
        "
        select cov_filepath, cov_line, state
        from ResolvedLineCoverageStates
        where product_id = $1
        ",
        product.id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|l| ResolvedLineCoverage {
        filepath: l.cov_filepath,
        line: l.cov_line,
        state: l.state.try_into().expect("Valid line state in database"),
    })
    .collect();

    let covered_traces: Vec<TraceReference> = sqlx::query!(
        "
        select distinct tc.filepath, tc.file_hash, traced_line, kind
        from TracesCoveredByTests tc, Traces t
        where tc.product_id = $1
        and tc.file_hash = t.file_hash and tc.traced_line = t.line
        ",
        product.id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|t| TraceReference {
        filepath: RelativePathBuf::from(t.filepath),
        file_hash: FmtHash::with_inner(t.file_hash),
        line: t.traced_line,
        kind: TraceKind::try_from(t.kind).expect("Valid trace kind in database"),
    })
    .collect();

    let covered_traces = if covered_traces.is_empty() {
        None
    } else {
        Some(covered_traces)
    };

    let mut resolved_files = HashMap::<String, HashMap<Line, ResolvedLineCoverage>>::new();

    for resolved_line in resolved_lines {
        let entry = resolved_files
            .entry(resolved_line.filepath.clone())
            .or_default();
        entry
            .entry(resolved_line.line)
            .and_modify(|_| {
                log::warn!(
                    "Multiple resolved line coverage entries for line '{}' in file '{}'",
                    resolved_line.line,
                    resolved_line.filepath
                )
            })
            .or_insert(resolved_line);
    }

    let mut covered_files = Vec::with_capacity(resolved_files.len());

    for (filepath, lines_map) in resolved_files {
        let file_record = sqlx::query!(
            "
            select file_hash
            from ProductRelatedFiles
            where product_id = $1 and filepath = $2
            ",
            product.id,
            filepath
        )
        .fetch_optional(transaction.as_mut())
        .await?;

        let lines: Vec<ResolvedLineCoverage> = lines_map.into_values().collect();

        let mut lines_summary = CoverageSummary::default();

        for line in &lines {
            match line.state {
                ResolvedLineCoverageState::Covered => lines_summary.covered.cnt += 1,
                ResolvedLineCoverageState::Excluded => lines_summary.excluded.cnt += 1,
                ResolvedLineCoverageState::OverriddenCovered => {
                    lines_summary.overridden_covered.cnt += 1
                }
                ResolvedLineCoverageState::OverriddenUncovered => {
                    lines_summary.overridden_uncovered.cnt += 1
                }
                ResolvedLineCoverageState::Uncovered => {
                    // Note: not relevant, because a test run may not have covered all possible files, so uncovered cnt is the diff from total to all other line states
                }
            }
        }

        test_summary.lines.add(&lines_summary);

        covered_files.push(TestCoveredFile {
            filepath: RelativePathBuf::from(filepath),
            file_hash: file_record.map(|f| FmtHash::with_inner(f.file_hash)),
        })
    }

    test_summary.lines.uncovered.cnt = test_summary.lines.total
        - (test_summary.lines.covered.cnt
            + test_summary.lines.excluded.cnt
            + test_summary.lines.overridden_covered.cnt
            + test_summary.lines.overridden_uncovered.cnt);

    test_summary.lines.update_percentages();

    Ok(TestCoverage {
        summary: test_summary,
        covered_files,
        covered_traces,
    })
}
