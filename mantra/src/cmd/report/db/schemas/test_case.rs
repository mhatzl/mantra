use std::{collections::HashMap, str::FromStr};

use mantra_schema::{
    FmtHash, Line, Properties, SCHEMA_VERSION,
    annotations::TraceKind,
    path::RelativePathBuf,
    product::ProductId,
    report::{
        annotations::TraceReference,
        formattable_content::FormattableContent,
        product::ProductReportSchema,
        requirement::RequirementReference,
        review::{ReviewReference, ReviewState},
        test_case::{TestCaseReference, TestCaseReportSchema},
        test_run::TestRunReference,
        tests::{
            CoverageSummary, ResolvedCoverageState, TestCoverage, TestCoverageSummary,
            TestCoveredFile, TestRelatedRequirement, TestRelatedRequirementKind,
        },
    },
    test_runs::{LogOutput, TestCaseLocation},
};

use crate::{
    cmd::report::db::schemas::{
        test_run::test_run_content_media_type, test_runs::ResolvedLineCoverage,
    },
    db::MantraTransaction,
};

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
                and tl.test_run_name = $2
                and tl.test_case_name = $3
        where tc.product_id = $1
        and tc.test_run_name = $2
        and tc.name = $3
        "#,
        product.id,
        test_case.test_run_name,
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

    let media_type =
        test_run_content_media_type(transaction, &product.id, &test_case.test_run_name).await?;

    let log_records = sqlx::query!(
        "
        select tl.log_src, gt.content
        from TestCaseLogs tl, GeneralTexts gt
        where tl.product_id = $1 and tl.test_run_name = $2
        and tl.test_case_name = $3
        and tl.log_hash = gt.hash
        ",
        product.id,
        test_case.test_run_name,
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
        and rp.test_case_name = $3
        and rp.value_hash = v.hash
        ",
        product.id,
        test_case.test_run_name,
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
        and rp.test_case_name = $3
        and rp.value_hash = v.hash
        ",
        product.id,
        test_case.test_run_name,
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
        select tso.review_name, rs.state
        from TestCaseOverrides tso, ReviewStates rs
        where tso.product_id = $1
        and tso.test_run_name = $2
        and tso.test_case_name = $3
        and rs.product_id = $1
        and rs.review_name = tso.review_name

        union

        select tco.review_name, rs.state
        from TestCaseLineCoverageOverrides tco, ReviewStates rs
        where tco.product_id = $1
        and tco.test_run_name = $2
        and tco.test_case_name = $3
        and rs.product_id = $1
        and rs.review_name = tco.review_name
        ",
        product.id,
        test_case.test_run_name,
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
                state: ReviewState::try_from(record.state)?,
            });
        }

        Some(overrides)
    };

    let coverage = test_case_related_coverage(transaction, product, test_case).await?;
    let related_reqs = test_case_related_requirements(transaction, product, test_case).await?;

    let schema_properties = test_case_schema_properties(
        transaction,
        &product.id,
        &test_case.test_run_name,
        &test_case.test_case_name,
    )
    .await?;
    let config_properties = test_case_config_properties(
        transaction,
        &product.id,
        &test_case.test_run_name,
        &test_case.test_case_name,
    )
    .await?;

    Ok(TestCaseReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        test_run: test_run.clone(),
        name: test_case.test_case_name.clone(),
        description: metadata
            .description
            .map(|d| FormattableContent::new(d, media_type)),
        state: test_case.state,
        state_properties,
        location,
        utc_date,
        duration_sec,
        logs,
        coverage,
        related_reqs,
        overridden_by,
        properties,
        schema_properties,
        config_properties,
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
            exists (
                select req_id
                from OptionalRequirements o
                where o.product_id = $1
                and o.req_id = dt.req_id
            ) as "optional!:bool",
            exists (
                select req_id
                from ManualRequirements m
                where m.product_id = $1
                and m.req_id = dt.req_id
            ) as "manual!:bool",
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
        and tc.test_run_name = $2
        and tc.test_case_name = $3
        and tc.filepath = dt.filepath and tc.file_hash = dt.file_hash
        and tc.traced_line = dt.line and dt.file_hash = t.file_hash
        and dt.line = t.line
        and rs.req_id = dt.req_id
        "#,
        product.id,
        test_case.test_run_name,
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
                manual: req_trace.manual,
            })
            .or_default()
            .push(TraceReference {
                path: RelativePathBuf::from(req_trace.filepath),
                line: Some(req_trace.line),
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
            exists (
                select req_id
                from OptionalRequirements o
                where o.product_id = $1
                and o.req_id = tc.req_id
            ) as "optional!:bool",
            exists (
                select req_id
                from ManualRequirements m
                where m.product_id = $1
                and m.req_id = tc.req_id
            ) as "manual!:bool"
        from TestCaseVerifiedRequirements tc, RequirementVerificationStates rs
        where tc.product_id = $1 and rs.product_id = $1
        and tc.test_run_name = $2
        and tc.test_case_name = $3
        and tc.req_id = rs.req_id
        "#,
        product.id,
        test_case.test_run_name,
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
                manual: direct_req.manual,
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
    let resolved_lines: Vec<_> = sqlx::query!(
        "
        select cov_filepath, cov_line, cov_state
        from ResolvedTestCaseLineCoverage
        where product_id = $1
        and test_run_name = $2
        and test_case_name = $3
        ",
        product.id,
        test_case.test_run_name,
        test_case.test_case_name
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|l| ResolvedLineCoverage {
        filepath: l.cov_filepath,
        line: l.cov_line,
        state: l
            .cov_state
            .try_into()
            .expect("Valid line state in database"),
    })
    .collect();

    let covered_traces: Vec<TraceReference> = sqlx::query!(
        "
        select distinct tc.filepath, tc.file_hash, traced_line, kind
        from TraceCoveragePerTestCases tc, Traces t
        where tc.product_id = $1 and tc.test_run_name = $2
        and tc.test_case_name = $3
        and tc.file_hash = t.file_hash and tc.traced_line = t.line
        ",
        product.id,
        test_case.test_run_name,
        test_case.test_case_name
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|t| TraceReference {
        path: RelativePathBuf::from(t.filepath),
        line: Some(t.traced_line),
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

    let mut covered_files = Vec::with_capacity(resolved_files.len());

    for (filepath, lines_map) in resolved_files {
        let file_record = sqlx::query!(
            "
            select file_hash
            from ProductRelatedHashedFiles
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
                ResolvedCoverageState::Covered => lines_summary.covered.cnt += 1,
                ResolvedCoverageState::Excluded => lines_summary.excluded.cnt += 1,
                ResolvedCoverageState::Uncovered => lines_summary.uncovered.cnt += 1,
                ResolvedCoverageState::Unknown => lines_summary.unknown.cnt += 1,
                ResolvedCoverageState::OverriddenCovered => {
                    lines_summary.overridden_covered.cnt += 1
                }
                ResolvedCoverageState::OverriddenUncovered => {
                    lines_summary.overridden_uncovered.cnt += 1
                }
                ResolvedCoverageState::OverriddenExcluded => {
                    lines_summary.overridden_excluded.cnt += 1
                }
            }
        }

        test_summary.lines.add(&lines_summary);

        covered_files.push(TestCoveredFile {
            filepath: RelativePathBuf::from(filepath),
            file_hash: file_record.map(|f| FmtHash::with_inner(f.file_hash)),
        })
    }

    // not every coverable line may have been reported by the test.
    // we treat those lines as "unknown"
    test_summary.lines.unknown.cnt += test_summary.lines.total
        - (test_summary.lines.covered.cnt
            + test_summary.lines.excluded.cnt
            + test_summary.lines.uncovered.cnt
            + test_summary.lines.unknown.cnt
            + test_summary.lines.overridden_covered.cnt
            + test_summary.lines.overridden_uncovered.cnt
            + test_summary.lines.overridden_excluded.cnt);

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

async fn test_case_schema_properties<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    test_run_name: &str,
    test_case_name: &str,
) -> Result<Option<Properties>, anyhow::Error> {
    let records = sqlx::query!(
        "
        select sp.property_key, gj.content
        from SchemaTestRunProperties sp, TestRuns tr, TestCases tc, GeneralJson gj
        where tr.product_id = $1 and tr.name = $2
        and tr.schema_hash = sp.schema_hash
        and tc.product_id = tr.product_id
        and tc.test_run_name = tr.name
        and tc.name = $3
        and gj.hash = sp.value_hash
        ",
        product_id,
        test_run_name,
        test_case_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    if records.is_empty() {
        return Ok(None);
    }

    let mut props = Properties::with_capacity(records.len());

    for record in records {
        props.insert(
            record.property_key,
            serde_json::Value::from_str(&record.content)?,
        );
    }

    Ok(Some(props))
}

async fn test_case_config_properties<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    test_run_name: &str,
    test_case_name: &str,
) -> Result<Option<Properties>, anyhow::Error> {
    let records = sqlx::query!(
        "
        select cp.property_key, gj.content
        from TestRuns tr, TestCases tc, ConfigCollectedSchemas cs, ConfigTestCaseProperties cp, GeneralJson gj
        where tr.product_id = $1 and tr.name = $2
        and tr.schema_hash = cs.schema_hash
        and cs.cfg_entry = cp.cfg_entry
        and tc.product_id = tr.product_id
        and tr.name = tc.test_run_name
        and tc.name = $3
        and gj.hash = cp.value_hash
        ",
        product_id,
        test_run_name,
        test_case_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    if records.is_empty() {
        return Ok(None);
    }

    let mut props = Properties::with_capacity(records.len());

    for record in records {
        props.insert(
            record.property_key,
            serde_json::Value::from_str(&record.content)?,
        );
    }

    Ok(Some(props))
}
