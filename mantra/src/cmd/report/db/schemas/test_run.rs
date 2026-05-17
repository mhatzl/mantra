use std::{collections::HashMap, str::FromStr};

use mantra_schema::{
    FmtHash, Line, Revision, SCHEMA_VERSION,
    annotations::TraceKind,
    path::RelativePathBuf,
    report::{
        annotations::TraceReference,
        product::ProductReportSchema,
        requirement::RequirementReference,
        test_case::TestCaseReference,
        test_run::{TestCasesOverview, TestRunReference, TestRunReportSchema},
        tests::{
            CoverageSummary, ResolvedLineCoverageState, TestCoverage, TestCoverageSummary,
            TestCoveredFile, TestRelatedRequirement, TestRelatedRequirementKind, TestState,
            TestsSummary,
        },
    },
    test_runs::LogOutput,
};

use crate::db::MantraTransaction;

pub async fn generate_test_run_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    test_run: &TestRunReference,
) -> Result<TestRunReportSchema, anyhow::Error> {
    let metadata = sqlx::query!(
        r#"
        select
            tr.nr_of_test_cases,
            tr.duration_sec,
            gt.content as "description?",
            og.content as "origin?",
            bo.content as "base_origin?"
        from TestRuns tr
        left join GeneralTexts gt on tr.description_hash = gt.hash
        left join GeneralJson og on tr.origin_hash = og.hash
        left join GeneralJson bo on tr.base_origin_hash = bo.hash
        where tr.product_id = $1 and tr.name = $2
        and tr.utc_date = $3
        "#,
        product.id,
        test_run.name,
        test_run.utc_date
    )
    .fetch_one(transaction.as_mut())
    .await?;

    let origin = match metadata.origin {
        Some(o) => Some(serde_json::Value::from_str(&o)?),
        None => None,
    };
    let base_origin = match metadata.base_origin {
        Some(o) => Some(serde_json::Value::from_str(&o)?),
        None => None,
    };
    let duration_sec = match metadata.duration_sec {
        Some(d) => Some(mantra_schema::time::Duration::saturating_seconds_f64(d)),
        None => None,
    };

    let revision_records = sqlx::query!(
        "
        select revision, comment
        from TestRunRevisions
        where product_id = $1 and test_run_name = $2
        and test_run_date = $3
        ",
        product.id,
        test_run.name,
        test_run.utc_date
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
                and test_run_date = $3 and revision = $4
                ",
                product.id,
                test_run.name,
                test_run.utc_date,
                rev.revision
            )
            .fetch_all(transaction.as_mut())
            .await?
            .into_iter()
            .map(|a| a.author)
            .collect();

            revisions.push(Revision {
                nr: rev.revision,
                authors,
                comment: rev.comment,
            });
        }

        Some(revisions)
    };

    let log_records = sqlx::query!(
        "
        select tl.log_src, gt.content
        from TestRunLogs tl, GeneralTexts gt
        where tl.product_id = $1 and tl.test_run_name = $2
        and tl.test_run_date = $3
        and tl.log_hash = gt.hash
        ",
        product.id,
        test_run.name,
        test_run.utc_date
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
        and rp.test_run_date = $3
        and rp.value_hash = v.hash
        ",
        product.id,
        test_run.name,
        test_run.utc_date
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
            select th.parent_name, th.parent_date, ts.state as "state!:i64"
            from TestRunHierarchies th, TestRunStates ts
            where th.product_id = $1 and ts.product_id = $1
            and th.child_name = $2 and th.child_date = $3
            and ts.test_run_name = th.parent_name
            and ts.test_run_date = th.parent_date
        "#,
        product.id,
        test_run.name,
        test_run.utc_date
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
                utc_date: mantra_schema::test_runs::test_date_from_str(&parent.parent_date)?,
                state: parent.state.try_into()?,
            })
        }

        Some(parents)
    };

    let children_records = sqlx::query!(
        r#"
            select th.child_name, th.child_date, ts.state as "state!:i64"
            from TestRunHierarchies th, TestRunStates ts
            where th.product_id = $1 and ts.product_id = $1
            and th.parent_name = $2 and th.parent_date = $3
            and ts.test_run_name = th.child_name
            and ts.test_run_date = th.child_date
        "#,
        product.id,
        test_run.name,
        test_run.utc_date
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
                utc_date: mantra_schema::test_runs::test_date_from_str(&child.child_date)?,
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
        and tc.test_run_date = $3 and rs.test_run_date = $3
        and tc.name = rs.test_case_name
        ",
        product.id,
        test_run.name,
        test_run.utc_date
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
            test_run_date: test_run.utc_date,
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

    let coverage = test_run_related_coverage(transaction, product, test_run).await?;
    let related_reqs = test_run_related_requirements(transaction, product, test_run).await?;

    Ok(TestRunReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        name: test_run.name.clone(),
        utc_date: test_run.utc_date,
        state: test_run.state,
        description: metadata.description,
        revisions,
        origin,
        base_origin,
        nr_of_test_cases: metadata.nr_of_test_cases,
        properties,
        duration_sec,
        logs,
        test_cases,
        child_test_runs,
        parent_test_runs,
        coverage,
        related_reqs,
    })
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
            TraceCoveragePerTestRuns tc,
            DirectProductReqTraces dt,
            Traces t,
            RequirementVerificationStates rs
        where tc.product_id = $1 and dt.product_id = $1
        and rs.product_id = $1
        and tc.test_run_name = $2 and tc.test_run_date = $3
        and tc.filepath = dt.filepath and tc.file_hash = dt.file_hash
        and tc.traced_line = dt.line and dt.file_hash = t.file_hash
        and dt.line = t.line
        and rs.id = dt.req_id
        "#,
        product.id,
        test_run.name,
        test_run.utc_date
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
            })
            .or_default()
            .push(TraceReference {
                filepath: RelativePathBuf::from(req_trace.filepath),
                file_hash: FmtHash::with_inner(req_trace.file_hash),
                line: req_trace.line,
                kind: TraceKind::try_from(req_trace.kind)?,
            });
    }

    let related_reqs: Vec<TestRelatedRequirement> = traced_reqs
        .into_iter()
        .map(|(req, traces)| TestRelatedRequirement {
            req: req,
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
        from ResolvedTestRunLineCoverage
        where product_id = $1
        and test_run_name = $2
        and test_run_date = $3
        ",
        product.id,
        test_run.name,
        test_run.utc_date
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
        from TraceCoveragePerTestRuns tc, Traces t
        where tc.product_id = $1 and tc.test_run_name = $2
        and tc.test_run_date = $3
        and tc.file_hash = t.file_hash and tc.traced_line = t.line
        ",
        product.id,
        test_run.name,
        test_run.utc_date
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
