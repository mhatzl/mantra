use std::{collections::HashMap, str::FromStr};

use mantra_schema::{
    FmtHash, Line, Origin, Properties, SCHEMA_VERSION,
    annotations::TraceKind,
    media_type::MediaType,
    mime_guess::Mime,
    path::RelativePathBuf,
    product::ProductId,
    report::{
        ResolvedRevision,
        annotations::TraceReference,
        formattable_content::FormattableContent,
        product::ProductReportSchema,
        requirement::RequirementReference,
        review::{ReviewReference, ReviewState},
        test_case::TestCaseReference,
        test_run::{TestCasesOverview, TestRunReference, TestRunReportSchema},
        tests::{
            CoverageSummary, ResolvedCoverageState, TestCoverage, TestCoverageSummary,
            TestCoveredFile, TestRelatedRequirement, TestRelatedRequirementKind, TestState,
            TestsSummary,
        },
    },
    test_runs::LogOutput,
};

use crate::{cmd::report::db::schemas::test_runs::ResolvedLineCoverage, db::MantraTransaction};

pub async fn generate_test_run_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    test_run: &TestRunReference,
) -> Result<TestRunReportSchema, anyhow::Error> {
    let metadata = sqlx::query!(
        r#"
        select
            tr.utc_date,
            tr.duration_sec,
            gt.content as "description?",
            og.content as "origin?"
        from TestRuns tr
        left join GeneralTexts gt on tr.description_hash = gt.hash
        left join GeneralJson og on tr.origin_hash = og.hash
        where tr.product_id = $1 and tr.name = $2
        "#,
        product.id,
        test_run.name,
    )
    .fetch_one(transaction.as_mut())
    .await?;

    let origin = match metadata.origin {
        Some(o) => Some(serde_json::Value::from_str(&o)?),
        None => None,
    };
    let duration_sec = metadata
        .duration_sec
        .map(mantra_schema::time::Duration::saturating_seconds_f64);

    let media_type = test_run_content_media_type(transaction, &product.id, &test_run.name).await?;

    let revision_records = sqlx::query!(
        "
        select tr.revision, gt.content
        from TestRunRevisions tr, GeneralTexts gt
        where tr.product_id = $1 and tr.test_run_name = $2
        and tr.summary_hash = gt.hash
        ",
        product.id,
        test_run.name,
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let revisions = if revision_records.is_empty() {
        None
    } else {
        let mut revisions = Vec::with_capacity(revision_records.len());

        for rev in revision_records {
            let authors = sqlx::query!(
                "
                select author
                from TestRunRevisionAuthors
                where product_id = $1 and test_run_name = $2
                and revision = $3
                ",
                product.id,
                test_run.name,
                rev.revision
            )
            .fetch_all(transaction.as_mut())
            .await?
            .into_iter()
            .map(|a| a.author)
            .collect();

            revisions.push(ResolvedRevision {
                number: rev.revision,
                authors,
                summary: FormattableContent::new(rev.content, media_type.clone()),
            });
        }

        Some(revisions)
    };

    let log_records = sqlx::query!(
        "
        select tl.log_src, gt.content
        from TestRunLogs tl, GeneralTexts gt
        where tl.product_id = $1 and tl.test_run_name = $2
        and tl.log_hash = gt.hash
        ",
        product.id,
        test_run.name,
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
        from TestRunProperties rp, GeneralJson v
        where rp.product_id = $1 and rp.test_run_name = $2
        and rp.value_hash = v.hash
        ",
        product.id,
        test_run.name,
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

    let parent_records = sqlx::query!(
        r#"
            select th.parent_name, ts.state as "state!:i64"
            from TestRunHierarchies th, TestRunStates ts
            where th.product_id = $1 and ts.product_id = $1
            and th.child_name = $2
            and ts.test_run_name = th.parent_name
        "#,
        product.id,
        test_run.name,
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let parent_test_runs = if parent_records.is_empty() {
        None
    } else {
        let mut parents = Vec::with_capacity(parent_records.len());

        for parent in parent_records {
            parents.push(TestRunReference {
                product_id: product.id.clone(),
                name: parent.parent_name,
                state: parent.state.try_into()?,
            })
        }

        Some(parents)
    };

    let children_records = sqlx::query!(
        r#"
            select th.child_name, ts.state as "state!:i64"
            from TestRunHierarchies th, TestRunStates ts
            where th.product_id = $1 and ts.product_id = $1
            and th.parent_name = $2
            and ts.test_run_name = th.child_name
        "#,
        product.id,
        test_run.name,
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let child_test_runs = if children_records.is_empty() {
        None
    } else {
        let mut children = Vec::with_capacity(children_records.len());

        for child in children_records {
            children.push(TestRunReference {
                product_id: product.id.clone(),
                name: child.child_name,
                state: child.state.try_into()?,
            })
        }

        Some(children)
    };

    let test_case_records = sqlx::query!(
        "
        select tc.name, rs.state
        from TestCases tc, ResolvedTestCaseStates rs
        where tc.product_id = $1 and rs.product_id = $1
        and tc.test_run_name = $2 and rs.test_run_name = $2
        and tc.name = rs.test_case_name
        ",
        product.id,
        test_run.name,
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut test_case_summary = TestsSummary {
        total: test_case_records.len() as i64,
        ..Default::default()
    };
    let mut passed = Vec::new();
    let mut failed = Vec::new();
    let mut skipped = Vec::new();
    let mut unknown = Vec::new();
    let mut obsolete = Vec::new();

    for record in test_case_records {
        let state: TestState = record.state.try_into()?;

        let test_case = TestCaseReference {
            product_id: product.id.clone(),
            test_run_name: test_run.name.clone(),
            test_case_name: record.name,
            state,
        };

        match state {
            TestState::Failed => failed.push(test_case),
            TestState::Passed => passed.push(test_case),
            TestState::Skipped => skipped.push(test_case),
            TestState::Unknown => unknown.push(test_case),
            TestState::Obsolete => obsolete.push(test_case),
        }
    }

    test_case_summary.failed.cnt = failed.len() as i64;
    test_case_summary.passed.cnt = passed.len() as i64;
    test_case_summary.skipped.cnt = skipped.len() as i64;
    test_case_summary.unknown.cnt = unknown.len() as i64;
    test_case_summary.obsolete.cnt = obsolete.len() as i64;

    test_case_summary.update_percentages();

    let test_cases = if test_case_summary.total > 0 {
        Some(TestCasesOverview {
            summary: test_case_summary,
            passed,
            failed,
            skipped,
            unknown,
            obsolete,
        })
    } else {
        None
    };

    let override_records = sqlx::query!(
        r#"
        select distinct tro.review_name, rs.state as "state!:i64"
        from TestRunLineCoverageOverrides tro, ReviewStates rs
        where tro.product_id = $1
        and tro.test_run_name = $2
        and rs.product_id = $1
        and rs.review_name = tro.review_name
        "#,
        product.id,
        test_run.name,
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

    let coverage = test_run_related_coverage(transaction, product, test_run).await?;
    let related_reqs = test_run_related_requirements(transaction, product, test_run).await?;

    let schema_origin = test_run_schema_origin(transaction, &product.id, &test_run.name).await?;
    let schema_properties =
        test_run_schema_properties(transaction, &product.id, &test_run.name).await?;

    let config_origin = test_run_config_origin(transaction, &product.id, &test_run.name).await?;
    let config_properties =
        test_run_config_properties(transaction, &product.id, &test_run.name).await?;

    Ok(TestRunReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        name: test_run.name.clone(),
        utc_date: mantra_schema::test_runs::test_date_from_str(&metadata.utc_date)?,
        state: test_run.state,
        description: metadata
            .description
            .map(|d| FormattableContent::new(d, media_type)),
        revisions,
        duration_sec,
        logs,
        test_cases,
        child_test_runs,
        parent_test_runs,
        coverage,
        related_reqs,
        overridden_by,
        origin,
        schema_origin,
        config_origin,
        properties,
        schema_properties,
        config_properties,
    })
}

pub(super) async fn test_run_content_media_type<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    test_run_name: &str,
) -> Result<Option<MediaType>, anyhow::Error> {
    let records = sqlx::query!(
        "
        select media_type
        from SchemaContentMediaTypes mt, TestRuns tr
        where tr.product_id = $1 and tr.name = $2
        and tr.schema_hash = mt.schema_hash
        order by mt.entry_number
        ",
        product_id,
        test_run_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    if records.is_empty() {
        return Ok(None);
    }

    let mut mimes = Vec::with_capacity(records.len());

    for record in records {
        let mime = Mime::from_str(&record.media_type)?;
        mimes.push(mime);
    }

    Ok(Some(MediaType::new(mimes)))
}

async fn test_run_related_requirements<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    test_run: &TestRunReference,
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
            TraceCoveragePerTestRuns tc,
            DirectProductReqTraces dt,
            Traces t,
            RequirementVerificationStates rs
        where tc.product_id = $1 and dt.product_id = $1
        and rs.product_id = $1
        and tc.test_run_name = $2
        and tc.filepath = dt.filepath and tc.file_hash = dt.file_hash
        and tc.traced_line = dt.line and dt.file_hash = t.file_hash
        and dt.line = t.line
        and rs.req_id = dt.req_id
        "#,
        product.id,
        test_run.name,
    )
    .fetch_all(transaction.as_mut())
    .await?;

    if req_traces.is_empty() {
        return Ok(None);
    }

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

    let related_reqs: Vec<TestRelatedRequirement> = traced_reqs
        .into_iter()
        .map(|(req, traces)| TestRelatedRequirement {
            req,
            kind: TestRelatedRequirementKind::Traced(traces),
        })
        .collect();

    Ok(Some(related_reqs))
}

async fn test_run_related_coverage<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    test_run: &TestRunReference,
) -> Result<Option<TestCoverage>, anyhow::Error> {
    let resolved_lines: Vec<_> = sqlx::query!(
        "
        select cov_filepath, cov_line, cov_state
        from ResolvedTestRunLineCoverage
        where product_id = $1
        and test_run_name = $2
        ",
        product.id,
        test_run.name,
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
        from TraceCoveragePerTestRuns tc, Traces t
        where tc.product_id = $1 and tc.test_run_name = $2
        and tc.file_hash = t.file_hash and tc.traced_line = t.line
        ",
        product.id,
        test_run.name,
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

async fn test_run_schema_origin<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    test_run_name: &str,
) -> Result<Option<Origin>, anyhow::Error> {
    let record = sqlx::query!(
        "
        select gj.content
        from Schemas s, TestRuns tr, GeneralJson gj
        where tr.product_id = $1 and tr.name = $2
        and tr.schema_hash = s.content_hash
        and gj.hash = s.origin_hash
        ",
        product_id,
        test_run_name
    )
    .fetch_optional(transaction.as_mut())
    .await?;

    match record {
        Some(r) => Ok(Some(serde_json::Value::from_str(&r.content)?)),
        None => Ok(None),
    }
}

async fn test_run_schema_properties<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    test_run_name: &str,
) -> Result<Option<Properties>, anyhow::Error> {
    let records = sqlx::query!(
        "
        select sp.property_key, gj.content
        from SchemaTestRunProperties sp, TestRuns tr, GeneralJson gj
        where tr.product_id = $1 and tr.name = $2
        and tr.schema_hash = sp.schema_hash
        and gj.hash = sp.value_hash
        ",
        product_id,
        test_run_name
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

async fn test_run_config_origin<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    test_run_name: &str,
) -> Result<Option<Origin>, anyhow::Error> {
    let record = sqlx::query!(
        "
        select gj.content
        from TestRuns tr, TestRunCollectConfigs trc, ConfigCollectedSchemas cs, GeneralJson gj
        where tr.product_id = $1 and tr.name = $2
        and tr.schema_hash = cs.schema_hash
        and cs.cfg_entry = trc.cfg_entry
        and gj.hash = trc.origin_hash
        ",
        product_id,
        test_run_name
    )
    .fetch_optional(transaction.as_mut())
    .await?;

    match record {
        Some(r) => Ok(Some(serde_json::Value::from_str(&r.content)?)),
        None => Ok(None),
    }
}

async fn test_run_config_properties<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    test_run_name: &str,
) -> Result<Option<Properties>, anyhow::Error> {
    let records = sqlx::query!(
        "
        select cp.property_key, gj.content
        from TestRuns tr, ConfigCollectedSchemas cs, ConfigTestRunProperties cp, GeneralJson gj
        where tr.product_id = $1 and tr.name = $2
        and tr.schema_hash = cs.schema_hash
        and cs.cfg_entry = cp.cfg_entry
        and gj.hash = cp.value_hash
        ",
        product_id,
        test_run_name
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
