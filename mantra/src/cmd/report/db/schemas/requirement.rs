use std::str::FromStr;

use mantra_schema::{
    SCHEMA_VERSION,
    annotations::TraceKind,
    media_type::MediaType,
    mime_guess::Mime,
    product::ProductId,
    report::{
        annotations::{TraceReference, TracesSummary},
        formattable_content::FormattableContent,
        product::ProductReportSchema,
        requirement::{
            RequirementAttributeState, RequirementCoverage, RequirementReference,
            RequirementReportSchema, RequirementReview, RequirementState,
            RequirementTracesOverview,
        },
        review::{ReviewReference, ReviewState},
        test_case::TestCaseReference,
        test_run::TestRunReference,
        tests::{TestReference, TestState},
    },
    requirements::ReqId,
};

use crate::{cmd::report::db::schemas::review::review_content_media_type, db::MantraTransaction};

pub async fn generate_requirement_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    req_id: &ReqId,
) -> Result<RequirementReportSchema, anyhow::Error> {
    let req = sqlx::query!(
        r#"
        select
            r.id,
            r.title,
            r.kind,
            rs.state,
            gt.content as "description?",
            og.content as "origin?",
            r.optional,
            r.manual_verification,
            r.deprecated,
            r.exclude,
            r.draft,
            exists (
                select o.req_id
                from OptionalRequirements o
                where o.product_id = $1 and o.req_id = r.id
            ) as "inherited_optional!:bool",
            exists (
                select m.req_id
                from ManualRequirements m
                where m.product_id = $1 and m.req_id = r.id
            ) as "inherited_manual_verification!:bool",
            exists (
                select dr.req_id
                from DeprecatedRequirements dr
                where dr.product_id = $1 and dr.req_id = r.id
            ) as "inherited_deprecated!:bool",
            exists (
                select er.req_id
                from ExcludedRequirements er
                where er.product_id = $1 and er.req_id = r.id
            ) as "inherited_exclude!:bool",
            exists (
                select dr.req_id
                from DraftRequirements dr
                where dr.product_id = $1 and dr.req_id = r.id
            ) as "inherited_draft!:bool"
        from RequirementVerificationStates rs, Requirements r
            left join GeneralTexts gt on r.description_hash = gt.hash
            left join GeneralJson og on r.origin_hash = og.hash
        where r.product_id = $1 and rs.product_id = $1
        and r.id = $2 and r.id = rs.req_id
        "#,
        product.id,
        req_id
    )
    .fetch_one(transaction.as_mut())
    .await?;

    let mut properties = serde_json::Map::new();

    for property in sqlx::query!(
        r#"
            select rp.property_key, gj.content
            from RequirementProperties rp, GeneralJson gj
            where rp.product_id = $1 and rp.req_id = $2
            and gj.hash = rp.value_hash
        "#,
        product.id,
        req_id
    )
    .fetch_all(transaction.as_mut())
    .await?
    {
        properties.insert(
            property.property_key,
            serde_json::Value::from_str(&property.content)?,
        );
    }

    let properties = if properties.is_empty() {
        None
    } else {
        Some(properties)
    };

    let children = requirement_child_references(transaction, &product.id, req_id).await?;
    let parents = requirement_parent_references(transaction, &product.id, req_id).await?;
    let reviewed_in = requirement_reviews(transaction, &product.id, req_id).await?;
    let covered_by = requirement_coverage(transaction, &product.id, req_id).await?;
    let traces = requirement_traces(transaction, &product.id, req_id).await?;
    let replaces = requirement_replacements(transaction, &product.id, req_id).await?;
    let replaced_by = requirement_replaced(transaction, &product.id, req_id).await?;

    let media_type = requirement_content_media_type(transaction, &product.id, req_id).await?;

    let description = req.description.map(|d| FormattableContent {
        content: d,
        media_type: media_type.clone(),
    });
    let title = FormattableContent {
        content: req.title,
        media_type,
    };

    Ok(RequirementReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        state: req.state.try_into()?,
        parents,
        children,
        traces,
        covered_by,
        reviewed_in,
        product: product.metadata(),
        id: req_id.clone(),
        kind: req.kind,
        title,
        description,
        manual_verification: requirement_attribute_state(
            req.manual_verification,
            req.inherited_manual_verification,
        ),
        deprecated: requirement_attribute_state(req.deprecated, req.inherited_deprecated),
        excluded: requirement_attribute_state(req.exclude, req.inherited_exclude),
        optional: requirement_attribute_state(req.optional, req.inherited_optional),
        draft: requirement_attribute_state(req.draft, req.inherited_draft),
        replaces,
        replaced_by,
        origin: req.origin.and_then(|o| serde_json::from_str(&o).ok()),
        schema_origin: todo!(),
        config_origin: todo!(),
        properties,
        schema_properties: todo!(),
        config_properties: todo!(),
    })
}

pub(super) async fn requirement_content_media_type<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Option<MediaType>, anyhow::Error> {
    let records = sqlx::query!(
        "
        select media_type
        from SchemaContentMediaTypes mt, Requirements r
        where r.product_id = $1 and r.id = $2
        and r.schema_hash = mt.schema_hash
        order by mt.entry_number
        ",
        product_id,
        req_id
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

pub(super) async fn requirement_coverage<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Option<Vec<RequirementCoverage>>, anyhow::Error> {
    let mut tests = coverage_by_test_runs(transaction, product_id, req_id).await?;
    tests.extend(coverage_by_test_cases(transaction, product_id, req_id).await?);

    if tests.is_empty() {
        Ok(None)
    } else {
        Ok(Some(tests))
    }
}

pub(super) async fn requirement_reviews<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Option<Vec<RequirementReview>>, anyhow::Error> {
    let records = sqlx::query!(
        "
        select mr.product_id, mr.review_name, rs.state, gt.content
        from ManuallyVerifiedRequirements mr, ReviewStates rs, GeneralTexts gt
        where mr.product_id = $1 and mr.req_id = $2
        and rs.product_id = $1 and rs.review_name = mr.review_name
        and mr.rational_hash = gt.hash
        ",
        product_id,
        req_id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut reviews = Vec::with_capacity(records.len());

    for record in records {
        let review_media_type =
            review_content_media_type(transaction, product_id, &record.review_name).await?;

        reviews.push(RequirementReview {
            review: ReviewReference {
                product_id: product_id.clone(),
                name: record.review_name,
                state: ReviewState::try_from(record.state)?,
            },
            rational: FormattableContent::new(record.content, review_media_type),
        })
    }

    if reviews.is_empty() {
        Ok(None)
    } else {
        Ok(Some(reviews))
    }
}

pub(super) async fn requirement_child_references<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Option<Vec<RequirementReference>>, anyhow::Error> {
    let child_records = sqlx::query!(
        r#"
        select
            r.product_id,
            r.id,
            rs.state,
            exists (
                select req_id
                from OptionalRequirements o
                where o.product_id = $1 and o.req_id = rs.req_id
            ) as "optional!:bool",
            exists (
                select req_id
                from ManualRequirements o
                where o.product_id = $1 and o.req_id = rs.req_id
            ) as "manual!:bool"
        from RequirementHierarchies rh, RequirementVerificationStates rs, Requirements r
        where rh.parent_product_id = $1 and rh.parent_req_id = $2
        and rh.child_product_id = rs.product_id and rh.child_req_id = rs.req_id
        and r.product_id = rs.product_id and r.id = rs.req_id
        "#,
        product_id,
        req_id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut children = Vec::with_capacity(child_records.len());
    for child in child_records {
        children.push(RequirementReference {
            id: child.id.try_into()?,
            product_id: child.product_id.try_into()?,
            state: child.state.try_into()?,
            optional: child.optional,
            manual: child.manual,
        })
    }

    if children.is_empty() {
        Ok(None)
    } else {
        Ok(Some(children))
    }
}

pub(super) async fn requirement_parent_references<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Option<Vec<RequirementReference>>, anyhow::Error> {
    let parent_records = sqlx::query!(
        r#"
        select
            r.product_id,
            r.id,
            rs.state,
            exists (
                select req_id
                from OptionalRequirements o
                where o.product_id = $1 and o.req_id = rs.req_id
            ) as "optional!:bool",
            exists (
                select req_id
                from ManualRequirements o
                where o.product_id = $1 and o.req_id = rs.req_id
            ) as "manual!:bool"
        from RequirementHierarchies rh, RequirementVerificationStates rs, Requirements r
        where rh.child_product_id = $1 and rh.child_req_id = $2
        and rh.parent_product_id = rs.product_id and rh.parent_req_id = rs.req_id
        and r.product_id = rs.product_id and r.id = rs.req_id
        "#,
        product_id,
        req_id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut parents = Vec::with_capacity(parent_records.len());
    for parent in parent_records {
        parents.push(RequirementReference {
            id: parent.id.try_into()?,
            product_id: parent.product_id.try_into()?,
            state: parent.state.try_into()?,
            optional: parent.optional,
            manual: parent.manual,
        })
    }

    if parents.is_empty() {
        Ok(None)
    } else {
        Ok(Some(parents))
    }
}

pub(super) async fn covering_test_runs<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Vec<TestRunReference>, anyhow::Error> {
    let covering_test_runs: Vec<TestRunReference> = sqlx::query!(
        r#"
        select distinct
            tr.product_id,
            tr.test_run_name,
            ts.state as "state!:i64"
        from
            TracesCoveredByTestRuns tr,
            DirectProductReqTraces dt,
            TestRunStates ts
        where tr.product_id = $1 and dt.product_id = $1 and ts.product_id = $1
        and tr.filepath = dt.filepath and tr.file_hash = dt.file_hash
        and tr.traced_line = dt.line and dt.req_id = $2
        and tr.test_run_name = ts.test_run_name
        "#,
        product_id,
        req_id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|tr| TestRunReference {
        product_id: product_id.clone(),
        name: tr.test_run_name,
        state: TestState::try_from(tr.state).expect("Valid test state in database"),
    })
    .collect();

    Ok(covering_test_runs)
}

pub(super) async fn covering_test_cases<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Vec<TestCaseReference>, anyhow::Error> {
    let covering_test_cases: Vec<TestCaseReference> = sqlx::query!(
        r#"
        select distinct
            tc.product_id,
            tc.test_run_name,
            tc.test_case_name,
            ts.state as "state!:i64"
        from
            TracesCoveredByTestCases tc,
            DirectProductReqTraces dt,
            ResolvedTestCaseStates ts
        where tc.product_id = $1 and dt.product_id = $1 and ts.product_id = $1
        and tc.filepath = dt.filepath and tc.file_hash = dt.file_hash
        and tc.traced_line = dt.line and dt.req_id = $2
        and tc.test_run_name = ts.test_run_name
        and tc.test_case_name = ts.test_case_name

        union

        select distinct
            tv.product_id,
            tv.test_run_name,
            tv.test_case_name,
            ts.state as "state!:i64"
        from TestCaseVerifiedRequirements tv, ResolvedTestCaseStates ts
        where tv.product_id = $1 and ts.product_id = $1
        and tv.req_id = $2
        and tv.test_run_name = ts.test_run_name
        and tv.test_case_name = ts.test_case_name
        "#,
        product_id,
        req_id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|tc| TestCaseReference {
        product_id: product_id.clone(),
        test_run_name: tc.test_run_name,
        test_case_name: tc.test_case_name,
        state: TestState::try_from(tc.state).expect("Valid test state in database"),
    })
    .collect();

    Ok(covering_test_cases)
}

async fn coverage_by_test_runs<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Vec<RequirementCoverage>, anyhow::Error> {
    let test_run_references = covering_test_runs(transaction, product_id, req_id).await?;

    let mut test_runs = Vec::with_capacity(test_run_references.len());

    for test_run in test_run_references {
        let covered_traces: Vec<_> = sqlx::query!(
            "
            select distinct dt.filepath, dt.file_hash, dt.line, t.kind
            from TracesCoveredByTestRuns tr, DirectProductReqTraces dt, Traces t
            where tr.product_id = $1 and dt.product_id = $1
            and dt.req_id = $2
            and tr.test_run_name = $3
            and dt.file_hash = t.file_hash
            and dt.line = t.line
            ",
            test_run.product_id,
            req_id,
            test_run.name,
        )
        .fetch_all(transaction.as_mut())
        .await?
        .into_iter()
        .map(|t| TraceReference {
            path: t.filepath.into(),
            line: Some(t.line),
            kind: t.kind.try_into().expect("Valid trace kind in database"),
        })
        .collect();

        test_runs.push(RequirementCoverage {
            test: TestReference::TestRun(TestRunReference {
                product_id: test_run.product_id,
                name: test_run.name,
                state: test_run.state,
            }),
            covered_traces: if covered_traces.is_empty() {
                None
            } else {
                Some(covered_traces)
            },
            directly_verified: false, // test run cannot directly verify a requirement
            missing_satisfy_trace: todo!(),
        });
    }

    Ok(test_runs)
}

async fn coverage_by_test_cases<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Vec<RequirementCoverage>, anyhow::Error> {
    let test_case_references = covering_test_cases(transaction, product_id, req_id).await?;

    let mut test_cases = Vec::with_capacity(test_case_references.len());

    for test_case in test_case_references {
        let covered_traces: Vec<_> = sqlx::query!(
            "
            select distinct dt.filepath, dt.file_hash, dt.line, t.kind
            from TracesCoveredByTestCases tc, DirectProductReqTraces dt, Traces t
            where tc.product_id = $1 and dt.product_id = $1
            and dt.req_id = $2
            and tc.test_run_name = $3
            and tc.test_case_name = $4
            and dt.file_hash = t.file_hash
            and dt.line = t.line
            ",
            test_case.product_id,
            req_id,
            test_case.test_run_name,
            test_case.test_case_name
        )
        .fetch_all(transaction.as_mut())
        .await?
        .into_iter()
        .map(|t| TraceReference {
            path: t.filepath.into(),
            line: Some(t.line),
            kind: t.kind.try_into().expect("Valid trace kind in database"),
        })
        .collect();

        let directly_verified = sqlx::query!(
            "
            select req_id
            from TestCaseVerifiedRequirements
            where product_id = $1 and req_id = $2
            and test_run_name = $3
            and test_case_name = $4
            ",
            test_case.product_id,
            req_id,
            test_case.test_run_name,
            test_case.test_case_name
        )
        .fetch_optional(transaction.as_mut())
        .await?
        .is_some();

        test_cases.push(RequirementCoverage {
            test: TestReference::TestCase(TestCaseReference {
                product_id: test_case.product_id,
                test_run_name: test_case.test_run_name,
                test_case_name: test_case.test_case_name,
                state: test_case.state,
            }),
            directly_verified,
            covered_traces: if covered_traces.is_empty() {
                None
            } else {
                Some(covered_traces)
            },
            missing_satisfy_trace: todo!(),
        });
    }

    Ok(test_cases)
}

async fn requirement_traces<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Option<RequirementTracesOverview>, anyhow::Error> {
    let trace_records = sqlx::query!(
        "
        select dt.filepath, dt.file_hash, dt.line, t.kind
        from DirectProductReqTraces dt, Traces t
        where dt.product_id = $1 and dt.req_id = $2
        and dt.file_hash = t.file_hash
        and dt.line = t.line
        ",
        product_id,
        req_id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    if trace_records.is_empty() {
        return Ok(None);
    }

    let mut traces = Vec::with_capacity(trace_records.len());
    let mut traces_summary = TracesSummary {
        total: trace_records.len() as i64,
        ..Default::default()
    };

    for trace in trace_records {
        let kind = TraceKind::try_from(trace.kind)?;

        match kind {
            TraceKind::Clarifies => traces_summary.clarifies.cnt += 1,
            TraceKind::Satisfies => traces_summary.satisfies.cnt += 1,
            TraceKind::Verifies => traces_summary.verifies.cnt += 1,
            TraceKind::Links => traces_summary.links.cnt += 1,
        }

        traces.push(TraceReference {
            path: trace.filepath.into(),
            line: Some(trace.line),
            kind,
        })
    }

    traces_summary.update_percentages();

    Ok(Some(RequirementTracesOverview {
        summary: traces_summary,
        all: traces,
    }))
}

async fn requirement_replacements<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Option<Vec<RequirementReference>>, anyhow::Error> {
    let replaced_records = sqlx::query!(
        r#"
        select
            rr.replaced_req_id,
            rs.state,
            exists (
                select o.req_id
                from OptionalRequirements o
                where o.product_id = $1 and o.req_id = rr.replaced_req_id
            ) as "optional!:bool",
            exists (
                select m.req_id
                from ManualRequirements m
                where m.product_id = $1 and m.req_id = rr.replaced_req_id
            ) as "manual!:bool"
        from RequirementReplacements rr, RequirementVerificationStates rs
        where rr.product_id = $1 and rr.req_id = $2
        and rs.product_id = $1 and rs.req_id = rr.replaced_req_id
        "#,
        product_id,
        req_id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    if replaced_records.is_empty() {
        return Ok(None);
    }

    let mut replaced_reqs = Vec::with_capacity(replaced_records.len());

    for record in replaced_records {
        replaced_reqs.push(RequirementReference {
            product_id: product_id.clone(),
            id: ReqId::from_str(&record.replaced_req_id)?,
            state: RequirementState::try_from(record.state)?,
            optional: record.optional,
            manual: record.manual,
        })
    }

    Ok(Some(replaced_reqs))
}

async fn requirement_replaced<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Option<Vec<RequirementReference>>, anyhow::Error> {
    let replaced_by_records = sqlx::query!(
        r#"
        select
            rr.req_id,
            rs.state,
            exists (
                select o.req_id
                from OptionalRequirements o
                where o.product_id = $1 and o.req_id = rr.req_id
            ) as "optional!:bool",
            exists (
                select m.req_id
                from ManualRequirements m
                where m.product_id = $1 and m.req_id = rr.req_id
            ) as "manual!:bool"
        from RequirementReplacements rr, RequirementVerificationStates rs
        where rr.product_id = $1 and rr.replaced_req_id = $2
        and rs.product_id = $1 and rs.req_id = rr.req_id
        "#,
        product_id,
        req_id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    if replaced_by_records.is_empty() {
        return Ok(None);
    }

    let mut replaced_by_reqs = Vec::with_capacity(replaced_by_records.len());

    for record in replaced_by_records {
        replaced_by_reqs.push(RequirementReference {
            product_id: product_id.clone(),
            id: ReqId::from_str(&record.req_id)?,
            state: RequirementState::try_from(record.state)?,
            optional: record.optional,
            manual: record.manual,
        })
    }

    Ok(Some(replaced_by_reqs))
}

fn requirement_attribute_state(own: Option<bool>, inherited: bool) -> RequirementAttributeState {
    match (own, inherited) {
        (Some(false), _) => RequirementAttributeState::False,
        (Some(true), _) => RequirementAttributeState::True,
        (None, false) => RequirementAttributeState::InheritedFalse,
        (None, true) => RequirementAttributeState::InheritedTrue,
    }
}
