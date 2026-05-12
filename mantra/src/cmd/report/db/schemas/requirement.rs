use mantra_schema::{
    FmtHash, SCHEMA_VERSION,
    product::ProductId,
    report::{
        annotations::TraceReference,
        product::ProductReportSchema,
        requirement::{
            RequirementCoverageByTestCases, RequirementCoverageByTestRuns,
            RequirementCoverageByTests, RequirementReference, RequirementReportSchema,
            RequirementReviewReference, RequirementTracesOverview,
        },
        test_case::TestCaseReference,
        test_run::TestRunReference,
        tests::TestState,
    },
    requirements::ReqId,
};

use crate::db::MantraTransaction;

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
            rs.state,
            gt.content as "description?",
            bo.content as "base_origin?",
            og.content as "origin?",
            case when exists (
                select o.id
                from OptionalRequirements o
                where o.product_id = $1 and o.id = r.id
            ) then true
            else false
            end as "optional!:bool",
            case when exists (
                select m.id
                from ManualRequirements m
                where m.product_id = $1 and m.id = r.id
            ) then true
            else false
            end as "manual_verification!:bool",
            case when exists (
                select dr.id
                from DeprecatedRequirements dr
                where dr.product_id = $1 and dr.id = r.id
            ) then true
            else false
            end as "deprecated!:bool",
            case when exists (
                select ir.id
                from IgnoredRequirements ir
                where ir.product_id = $1 and ir.id = r.id
            ) then true
            else false
            end as "ignored!:bool"
        from Requirements r
            left join GeneralTexts gt on r.description_hash = gt.hash
            left join GeneralJson og on r.origin_hash = og.hash
            left join GeneralJson bo on r.base_origin_hash = bo.hash,
        RequirementVerificationStates rs
        where r.product_id = $1 and rs.product_id = $1
        and r.id = $2 and r.id = rs.id
        "#,
        product.id,
        req_id
    )
    .fetch_one(transaction.as_mut())
    .await?;

    let children = requirement_child_references(transaction, &product.id, req_id).await?;
    let children = if children.is_empty() {
        None
    } else {
        Some(children)
    };

    let parents = requirement_parent_references(transaction, &product.id, req_id).await?;
    let parents = if parents.is_empty() {
        None
    } else {
        Some(parents)
    };

    let reviewed_in: Vec<RequirementReviewReference> = sqlx::query!(
        "
        select mr.product_id, mr.review_name, mr.review_date, gt.content
        from ManuallyVerifiedRequirements mr, GeneralTexts gt
        where mr.product_id = $1 and mr.req_id = $2
        and mr.comment_hash = gt.hash
        ",
        product.id,
        req_id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|r| RequirementReviewReference {
        product_id: product.id.clone(),
        name: r.review_name,
        utc_date: mantra_schema::reviews::date_from_str(&r.review_date)
            .expect("Valid review date in database"),
        state: mantra_schema::report::review::ReviewState::Valid, // TODO: replace with actual state
        comment: r.content,
    })
    .collect();
    let reviewed_in = if reviewed_in.is_empty() {
        None
    } else {
        Some(reviewed_in)
    };

    let test_runs = coverage_by_test_runs(transaction, &product.id, req_id).await?;
    let test_cases = coverage_by_test_cases(transaction, &product.id, req_id).await?;

    let covered_by = if test_runs.is_empty() && test_cases.is_empty() {
        None
    } else {
        Some(RequirementCoverageByTests {
            test_runs,
            test_cases,
        })
    };

    Ok(RequirementReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        state: req.state.try_into()?,
        parents,
        children,
        traces: None, //TODO
        covered_by,
        reviewed_in,
        product: product.metadata(),
        id: req_id.clone(),
        title: req.title,
        description: req.description,
        base_origin: req.base_origin.and_then(|o| serde_json::from_str(&o).ok()),
        origin: req.origin.and_then(|o| serde_json::from_str(&o).ok()),
        manual_verification: req.manual_verification,
        deprecated: req.deprecated,
        ignored: req.ignored,
        optional: req.optional,
        properties: None, //TODO
    })
}

pub(super) async fn requirement_child_references<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Vec<RequirementReference>, anyhow::Error> {
    let child_records = sqlx::query!(
        r#"
        select
            r.product_id,
            r.id,
            rs.state,
            case when exists (
                select o.id
                from OptionalRequirements o
                where o.product_id = $1 and o.id = r.id
            ) then true
            else false
            end as "optional!:bool"
        from RequirementHierarchies rh, RequirementVerificationStates rs, Requirements r
        where rh.parent_product_id = $1 and rh.parent_req_id = $2
        and rh.child_product_id = rs.product_id and rh.child_req_id = rs.id
        and r.product_id = rs.product_id and r.id = rs.id
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
        })
    }

    Ok(children)
}

pub(super) async fn requirement_parent_references<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Vec<RequirementReference>, anyhow::Error> {
    let parent_records = sqlx::query!(
        r#"
        select
            r.product_id,
            r.id,
            rs.state,
            case when exists (
                select o.id
                from OptionalRequirements o
                where o.product_id = $1 and o.id = r.id
            ) then true
            else false
            end as "optional!:bool"
        from RequirementHierarchies rh, RequirementVerificationStates rs, Requirements r
        where rh.child_product_id = $1 and rh.child_req_id = $2
        and rh.parent_product_id = rs.product_id and rh.parent_req_id = rs.id
        and r.product_id = rs.product_id and r.id = rs.id
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
        })
    }

    Ok(parents)
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
            tr.test_run_date,
            ts.state as "state!:i64"
        from
            TracesCoveredByTestRuns tr,
            DirectProductReqTraces dt,
            TestRunStates ts
        where tr.product_id = $1 and dt.product_id = $1 and ts.product_id = $1
        and tr.filepath = dt.filepath and tr.file_hash = dt.file_hash
        and tr.traced_line = dt.line and dt.req_id = $2
        and tr.test_run_name = ts.test_run_name
        and tr.test_run_date = ts.test_run_date
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
        utc_date: mantra_schema::test_runs::test_date_from_str(&tr.test_run_date)
            .expect("Valid test date in database"),
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
            tc.test_run_date,
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
        and tc.test_run_date = ts.test_run_date
        and tc.test_case_name = ts.test_case_name

        union

        select distinct
            tv.product_id,
            tv.test_run_name,
            tv.test_run_date,
            tv.test_case_name,
            ts.state as "state!:i64"
        from TestCaseVerifiedRequirements tv, ResolvedTestCaseStates ts
        where tv.product_id = $1 and ts.product_id = $1
        and tv.req_id = $2
        and tv.test_run_name = ts.test_run_name
        and tv.test_run_date = ts.test_run_date
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
        test_run_date: mantra_schema::test_runs::test_date_from_str(&tc.test_run_date)
            .expect("Valid test date in database"),
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
) -> Result<Vec<RequirementCoverageByTestRuns>, anyhow::Error> {
    let test_run_references = covering_test_runs(transaction, product_id, req_id).await?;

    let mut test_runs = Vec::with_capacity(test_run_references.len());

    for test_run in test_run_references {
        let covered_traces: Vec<_> = sqlx::query!(
            "
            select distinct dt.filepath, dt.file_hash, dt.line, t.kind
            from TracesCoveredByTestRuns tr, DirectProductReqTraces dt, Traces t
            where tr.product_id = $1 and dt.product_id = $1
            and dt.req_id = $2
            and tr.test_run_name = $3 and tr.test_run_date = $4
            and dt.file_hash = t.file_hash and dt.line = t.line
            ",
            test_run.product_id,
            req_id,
            test_run.name,
            test_run.utc_date
        )
        .fetch_all(transaction.as_mut())
        .await?
        .into_iter()
        .map(|t| TraceReference {
            filepath: t.filepath.into(),
            file_hash: FmtHash::with_inner(t.file_hash),
            line: t.line,
            kind: t.kind.try_into().expect("Valid trace kind in database"),
        })
        .collect();

        test_runs.push(RequirementCoverageByTestRuns {
            product_id: test_run.product_id,
            name: test_run.name,
            utc_date: test_run.utc_date,
            state: test_run.state,
            covered_traces: if covered_traces.is_empty() {
                None
            } else {
                Some(covered_traces)
            },
        });
    }

    Ok(test_runs)
}

async fn coverage_by_test_cases<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<Vec<RequirementCoverageByTestCases>, anyhow::Error> {
    let test_case_references = covering_test_cases(transaction, product_id, req_id).await?;

    let mut test_cases = Vec::with_capacity(test_case_references.len());

    for test_case in test_case_references {
        let covered_traces: Vec<_> = sqlx::query!(
            "
            select distinct dt.filepath, dt.file_hash, dt.line, t.kind
            from TracesCoveredByTestCases tc, DirectProductReqTraces dt, Traces t
            where tc.product_id = $1 and dt.product_id = $1
            and dt.req_id = $2
            and tc.test_run_name = $3 and tc.test_run_date = $4
            and tc.test_case_name = $6
            and dt.file_hash = t.file_hash and dt.line = t.line
            ",
            test_case.product_id,
            req_id,
            test_case.test_run_name,
            test_case.test_run_date,
            test_case.test_case_name
        )
        .fetch_all(transaction.as_mut())
        .await?
        .into_iter()
        .map(|t| TraceReference {
            filepath: t.filepath.into(),
            file_hash: FmtHash::with_inner(t.file_hash),
            line: t.line,
            kind: t.kind.try_into().expect("Valid trace kind in database"),
        })
        .collect();

        let directly_verified = sqlx::query!(
            "
            select req_id
            from TestCaseVerifiedRequirements
            where product_id = $1 and req_id = $2
            and test_run_name = $3 and test_run_date = $4
            and test_case_name = $5
            ",
            test_case.product_id,
            req_id,
            test_case.test_run_name,
            test_case.test_run_date,
            test_case.test_case_name
        )
        .fetch_optional(transaction.as_mut())
        .await?
        .is_some();

        test_cases.push(RequirementCoverageByTestCases {
            product_id: test_case.product_id,
            test_run_name: test_case.test_run_name,
            test_run_date: test_case.test_run_date,
            test_case_name: test_case.test_case_name,
            state: test_case.state,
            directly_verified,
            covered_traces: if covered_traces.is_empty() {
                None
            } else {
                Some(covered_traces)
            },
        });
    }

    Ok(test_cases)
}

async fn requirement_traces<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req_id: &ReqId,
) -> Result<RequirementTracesOverview, anyhow::Error> {
    todo!()
}
