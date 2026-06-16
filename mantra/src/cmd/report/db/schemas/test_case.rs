use std::{collections::HashMap, str::FromStr};

use mantra_schema::{
    FmtHash, Line, SCHEMA_VERSION,
    annotations::TraceKind,
    path::RelativePathBuf,
    report::{
        annotations::TraceReference,
        product::ProductReportSchema,
        requirement::RequirementReference,
        review::ReviewReference,
        test_case::{TestCaseReference, TestCaseReportSchema},
        test_run::TestRunReference,
        tests::{
            CoverageSummary, ResolvedLineCoverageState, TestCoverage, TestCoverageSummary,
            TestCoveredFile, TestRelatedRequirement, TestRelatedRequirementKind,
        },
    },
    test_runs::{LogOutput, TestCaseLocation},
};

use crate::{cmd::report::db::schemas::test_runs::ResolvedLineCoverage, db::MantraTransaction};

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
    let duration_sec = metadata
        .duration_sec
        .map(mantra_schema::time::Duration::saturating_seconds_f64);
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

    let override_records = sqlx::query!(
        "
        select tso.review_name, tso.review_date
        from TestCaseOverrides tso
        where tso.product_id = $1
        and tso.test_run_name = $2
        and tso.test_run_date = $3
        and tso.test_case_name = $4

        union

        select tco.review_name, tco.review_date
        from TestCaseLineCoverageOverrides tco
        where tco.product_id = $1
        and tco.test_run_name = $2
        and tco.test_run_date = $3
        and tco.test_case_name = $4
        ",
        product.id,
        test_case.test_run_name,
        test_case.test_run_date,
        test_case.test_case_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let overridden_by = if override_records.is_empty() {
        None
    } else {
        let mut overrides = Vec::with_capacity(override_records.len());

        for record in override_records {
            overrides.push(ReviewReference {
                product_id: product.id.clone(),
                name: record.review_name,
                utc_date: mantra_schema::reviews::date_from_str(&record.review_date)?,
                state: mantra_schema::report::review::ReviewState::Valid, //TODO set correct state
            });
        }

        Some(overrides)
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
        overridden_by,
    })
}

async fn test_case_related_requirements<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    test_case: &TestCaseReference,
) -> Result<Option<Vec<TestRelatedRequirement>>, anyhow::Error> {
    let req_traces = sqlx::query!(
        r#"
        select distinct
            dt.req_id,
            rs.state,
            case
                when exists (
                    select id
                    from OptionalRequirements o
                    where o.product_id = $1
                    and o.id = dt.req_id
                ) then true
                else false
            end as "optional!:bool",
            dt.filepath,
            dt.file_hash,
            dt.line,
            t.kind
        from
            TraceCoveragePerTestCases tc,
            DirectProductReqTraces dt,
            Traces t,
            RequirementVerificationStates rs
        where tc.product_id = $1 and dt.product_id = $1
        and rs.product_id = $1
        and tc.test_run_name = $2 and tc.test_run_date = $3
        and tc.test_case_name = $4
        and tc.filepath = dt.filepath and tc.file_hash = dt.file_hash
        and tc.traced_line = dt.line and dt.file_hash = t.file_hash
        and dt.line = t.line
        and rs.id = dt.req_id
        "#,
        product.id,
        test_case.test_run_name,
        test_case.test_run_date,
        test_case.test_case_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut traced_reqs: HashMap<RequirementReference, Vec<TraceReference>> = HashMap::new();

    for req_trace in req_traces {
        traced_reqs
            .entry(RequirementReference {
                product_id: product.id.clone(),
                id: req_trace.req_id.try_into()?,
                state: req_trace.state.try_into()?,
                optional: req_trace.optional,
            })
            .or_default()
            .push(TraceReference {
                filepath: RelativePathBuf::from(req_trace.filepath),
                file_hash: FmtHash::with_inner(req_trace.file_hash),
                line: req_trace.line,
                kind: TraceKind::try_from(req_trace.kind)?,
            });
    }

    let mut related_reqs: Vec<TestRelatedRequirement> = traced_reqs
        .into_iter()
        .map(|(req, traces)| TestRelatedRequirement {
            req,
            kind: TestRelatedRequirementKind::Traced(traces),
        })
        .collect();

    let directly_verified_reqs = sqlx::query!(
        r#"
        select
            tc.req_id,
            rs.state,
            case
                when exists (
                    select id
                    from OptionalRequirements o
                    where o.product_id = $1
                    and o.id = tc.req_id
                ) then true
                else false
            end as "optional!:bool"
        from TestCaseVerifiedRequirements tc, RequirementVerificationStates rs
        where tc.product_id = $1 and rs.product_id = $1
        and tc.test_run_name = $2 and tc.test_run_date = $3
        and tc.test_case_name = $4
        and tc.req_id = rs.id
        "#,
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
            req: RequirementReference {
                product_id: product.id.clone(),
                id: direct_req.req_id.try_into()?,
                state: direct_req.state.try_into()?,
                optional: direct_req.optional,
            },
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
