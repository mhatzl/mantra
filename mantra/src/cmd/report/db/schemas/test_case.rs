use std::{collections::HashMap, str::FromStr};

use mantra_schema::{
    FmtHash, Line, SCHEMA_VERSION,
    annotations::TraceKind,
    path::RelativePathBuf,
    report::{
        annotations::TraceReference,
        product::ProductReportSchema,
        test_case::{TestCaseReference, TestCaseReportSchema},
        test_run::TestRunReference,
        tests::{
            CoverageSummary, ResolvedLineCoverageState, TestCoverage, TestCoverageSummary,
            TestCoveredFile, TestRelatedRequirement, TestRelatedRequirementKind,
        },
    },
    requirements::ReqId,
    test_runs::{LogOutput, TestCaseLocation},
};

use crate::db::MantraTransaction;

pub async fn generate_test_case_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    test_run: &TestRunReference,
    test_case: &TestCaseReference,
) -> Result<TestCaseReportSchema, anyhow::Error> {
    let metadata = sqlx::query!(
        r#"
        select
            tc.utc_date,
            tc.duration_sec,
            gt.content as "description?",
            tl.filepath as "location_filepath?",
            tl.file_hash as "location_file_hash?",
            tl.line as "location_line?"
        from TestCases tc
            left join GeneralTexts gt on tc.description_hash = gt.hash
            left join TestCaseLocations tl on tl.product_id = $1
                and tl.test_run_name = $2 and tl.test_run_date = $3
                and tl.test_case_name = $4
        where tc.product_id = $1
        and tc.test_run_name = $2
        and tc.test_run_date = $3
        and tc.name = $4
        "#,
        product.id,
        test_case.test_run_name,
        test_case.test_run_date,
        test_case.test_case_name
    )
    .fetch_one(transaction.as_mut())
    .await?;

    let utc_date = metadata
        .utc_date
        .as_deref()
        .and_then(|d| mantra_schema::test_runs::test_date_from_str(d).ok());
    let duration_sec = match metadata.duration_sec {
        Some(d) => Some(mantra_schema::time::Duration::saturating_seconds_f64(d)),
        None => None,
    };
    let location = if let Some(filepath) = metadata.location_filepath
        && let Some(line) = metadata.location_line
    {
        Some(TestCaseLocation {
            filepath: filepath.into(),
            file_hash: metadata.location_file_hash.map(FmtHash::with_inner),
            line,
        })
    } else {
        None
    };

    let log_records = sqlx::query!(
        "
        select tl.log_src, gt.content
        from TestCaseLogs tl, GeneralTexts gt
        where tl.product_id = $1 and tl.test_run_name = $2
        and tl.test_run_date = $3 and tl.test_case_name = $4
        and tl.log_hash = gt.hash
        ",
        product.id,
        test_case.test_run_name,
        test_case.test_run_date,
        test_case.test_case_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let logs = if log_records.is_empty() {
        None
    } else {
        let mut logs = Vec::with_capacity(log_records.len());

        for log in log_records {
            logs.push(LogOutput {
                source: log.log_src.try_into()?,
                content: log.content,
            });
        }

        Some(logs)
    };

    let property_records = sqlx::query!(
        "
        select rp.property_key, v.content
        from TestCaseProperties rp, GeneralJson v
        where rp.product_id = $1 and rp.test_run_name = $2
        and rp.test_run_date = $3 and rp.test_case_name = $4
        and rp.value_hash = v.hash
        ",
        product.id,
        test_case.test_run_name,
        test_case.test_run_date,
        test_case.test_case_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let properties = if property_records.is_empty() {
        None
    } else {
        let mut properties = serde_json::value::Map::with_capacity(property_records.len());

        for prop in property_records {
            properties.insert(
                prop.property_key,
                serde_json::Value::from_str(&prop.content)?,
            );
        }

        Some(properties)
    };

    let state_property_records = sqlx::query!(
        "
        select rp.property_key, v.content
        from TestCaseStateProperties rp, GeneralJson v
        where rp.product_id = $1 and rp.test_run_name = $2
        and rp.test_run_date = $3 and rp.test_case_name = $4
        and rp.value_hash = v.hash
        ",
        product.id,
        test_case.test_run_name,
        test_case.test_run_date,
        test_case.test_case_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let state_properties = if state_property_records.is_empty() {
        None
    } else {
        let mut state_properties =
            serde_json::value::Map::with_capacity(state_property_records.len());

        for state_prop in state_property_records {
            state_properties.insert(
                state_prop.property_key,
                serde_json::Value::from_str(&state_prop.content)?,
            );
        }

        Some(state_properties)
    };

    let coverage = test_case_related_coverage(transaction, product, test_case).await?;
    let related_reqs = test_case_related_requirements(transaction, product, test_case).await?;

    Ok(TestCaseReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        test_run: test_run.clone(),
        name: test_case.test_case_name.clone(),
        description: metadata.description,
        state: test_case.state,
        state_properties,
        location,
        utc_date,
        duration_sec,
        properties,
        logs,
        coverage,
        related_reqs,
    })
}

async fn test_case_related_requirements<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    test_case: &TestCaseReference,
) -> Result<Option<Vec<TestRelatedRequirement>>, anyhow::Error> {
    let req_traces = sqlx::query!(
        "
        select distinct dt.req_id, dt.filepath, dt.file_hash, dt.line, t.kind
        from TraceCoveragePerTestCases tc, DirectProductReqTraces dt, Traces t
        where tc.product_id = $1 and dt.product_id = $1
        and tc.test_run_name = $2 and tc.test_run_date = $3
        and tc.test_case_name = $4
        and tc.filepath = dt.filepath and tc.file_hash = dt.file_hash
        and tc.traced_line = dt.line and dt.file_hash = t.file_hash
        and dt.line = t.line
        ",
        product.id,
        test_case.test_run_name,
        test_case.test_run_date,
        test_case.test_case_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut traced_reqs: HashMap<ReqId, Vec<TraceReference>> = HashMap::new();

    for trace in req_traces {
        traced_reqs
            .entry(trace.req_id.try_into()?)
            .or_default()
            .push(TraceReference {
                filepath: RelativePathBuf::from(trace.filepath),
                file_hash: FmtHash::with_inner(trace.file_hash),
                line: trace.line,
                kind: TraceKind::try_from(trace.kind)?,
            });
    }

    let mut related_reqs: Vec<TestRelatedRequirement> = traced_reqs
        .into_iter()
        .map(|(req_id, traces)| TestRelatedRequirement {
            product_id: product.id.clone(),
            id: req_id,
            kind: TestRelatedRequirementKind::Traced(traces),
        })
        .collect();

    let directly_verified_reqs = sqlx::query!(
        "
        select req_id
        from TestCaseVerifiedRequirements
        where product_id = $1
        and test_run_name = $2 and test_run_date = $3
        and test_case_name = $4
        ",
        product.id,
        test_case.test_run_name,
        test_case.test_run_date,
        test_case.test_case_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    related_reqs.reserve(directly_verified_reqs.len());

    for direct_req in directly_verified_reqs {
        related_reqs.push(TestRelatedRequirement {
            product_id: product.id.clone(),
            id: direct_req.req_id.try_into()?,
            kind: TestRelatedRequirementKind::Direct,
        })
    }

    Ok(if related_reqs.is_empty() {
        None
    } else {
        Some(related_reqs)
    })
}

async fn test_case_related_coverage<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    test_case: &TestCaseReference,
) -> Result<Option<TestCoverage>, anyhow::Error> {
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
        from ResolvedTestCaseLineCoverage
        where product_id = $1
        and test_run_name = $2
        and test_run_date = $3
        and test_case_name = $4
        ",
        product.id,
        test_case.test_run_name,
        test_case.test_run_date,
        test_case.test_case_name
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
        from TraceCoveragePerTestCases tc, Traces t
        where tc.product_id = $1 and tc.test_run_name = $2
        and tc.test_run_date = $3 and tc.test_case_name = $4
        and tc.file_hash = t.file_hash and tc.traced_line = t.line
        ",
        product.id,
        test_case.test_run_name,
        test_case.test_run_date,
        test_case.test_case_name
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

        let lines_record = sqlx::query!(
            r#"
            select coverable_lines
            from CoverableLinesPerFilepath
            where product_id = $1 and filepath = $2
            "#,
            product.id,
            filepath
        )
        .fetch_one(transaction.as_mut())
        .await?;

        let mut lines_summary = CoverageSummary {
            total: lines_record.coverable_lines,
            ..Default::default()
        };

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
                ResolvedLineCoverageState::Uncovered => lines_summary.uncovered.cnt += 1,
            }
        }

        let uncovered_cnt = lines_summary.total
            - (lines_summary.covered.cnt
                + lines_summary.excluded.cnt
                + lines_summary.overridden_covered.cnt
                + lines_summary.overridden_uncovered.cnt);
        if lines_summary.uncovered.cnt < uncovered_cnt {
            log::warn!(
                "Missing line coverage data in file '{}' for '{}' lines.",
                &filepath,
                uncovered_cnt - lines_summary.uncovered.cnt
            );
        } else if lines_summary.uncovered.cnt > uncovered_cnt {
            log::warn!(
                "Too many line coverage entries in file '{}' for '{}' lines.",
                &filepath,
                lines_summary.uncovered.cnt - uncovered_cnt
            );
        }

        test_summary.lines.add(&lines_summary);

        covered_files.push(TestCoveredFile {
            filepath: RelativePathBuf::from(filepath),
            file_hash: file_record.map(|f| FmtHash::with_inner(f.file_hash)),
        })
    }

    test_summary.lines.update_percentages();

    Ok(if covered_files.is_empty() {
        None
    } else {
        Some(TestCoverage {
            summary: test_summary,
            covered_files,
            covered_traces,
        })
    })
}

struct ResolvedLineCoverage {
    filepath: String,
    line: Line,
    state: ResolvedLineCoverageState,
}
