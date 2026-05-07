use mantra_schema::{
    SCHEMA_VERSION,
    product::ProductId,
    report::{
        evidence_matrix::{
            EvidenceMatrixSchema, RequirementCoverageByTestsOverview, RequirementEvidence,
        },
        product::ProductReportSchema,
        requirement::RequirementReference,
        review::ReviewReference,
        test_case::TestCaseReference,
        test_run::TestRunReference,
        tests::TestState,
    },
    requirements::ReqId,
};

use crate::db::MantraTransaction;

pub async fn generate_evidence_matrix_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
) -> Result<EvidenceMatrixSchema, anyhow::Error> {
    let requirements = sqlx::query_as!(
        RequirementDbMetadata,
        r#"
        select
            r.id,
            r.title,
            rs.state,
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
            end as "manual_verification!:bool"
        from Requirements r, RequirementVerificationStates rs
        where r.product_id = $1 and rs.product_id = $1
        and r.id = rs.id
        "#,
        product.id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut evidence = Vec::with_capacity(requirements.len());

    for req in requirements {
        evidence.push(evidence_per_requirement(transaction, &product.id, req).await?);
    }

    Ok(EvidenceMatrixSchema {
        version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        requirements: evidence,
    })
}

struct RequirementDbMetadata {
    id: ReqId,
    title: String,
    state: i64,
    optional: bool,
    manual_verification: bool,
}

async fn evidence_per_requirement<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    req: RequirementDbMetadata,
) -> Result<RequirementEvidence, anyhow::Error> {
    let children_record = sqlx::query!(
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
        req.id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut children = Vec::with_capacity(children_record.len());
    for child in children_record {
        children.push(RequirementReference {
            id: child.id,
            product_id: child.product_id,
            state: child.state.try_into()?,
            optional: child.optional,
        })
    }

    let parent_record = sqlx::query!(
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
        req.id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut parents = Vec::with_capacity(parent_record.len());
    for parent in parent_record {
        parents.push(RequirementReference {
            id: parent.id,
            product_id: parent.product_id,
            state: parent.state.try_into()?,
            optional: parent.optional,
        })
    }

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
        req.id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|tr| TestRunReference {
        product_id: tr.product_id,
        name: tr.test_run_name,
        utc_date: mantra_schema::test_runs::test_date_from_str(&tr.test_run_date)
            .expect("Valid test date in database"),
        state: TestState::try_from(tr.state).expect("Valid test state in database"),
    })
    .collect();

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
        req.id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|tc| TestCaseReference {
        product_id: tc.product_id,
        test_run_name: tc.test_run_name,
        test_run_date: mantra_schema::test_runs::test_date_from_str(&tc.test_run_date)
            .expect("Valid test date in database"),
        test_case_name: tc.test_case_name,
        state: TestState::try_from(tc.state).expect("Valid test state in database"),
    })
    .collect();

    let reviewed_in: Vec<ReviewReference> = sqlx::query!(
        "
        select product_id, review_name, review_date
        from ManuallyVerifiedRequirements
        where product_id = $1 and req_id = $2
        ",
        product_id,
        req.id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|r| ReviewReference {
        product_id: r.product_id,
        name: r.review_name,
        utc_date: mantra_schema::reviews::date_from_str(&r.review_date)
            .expect("Valid review date in database"),
        state: mantra_schema::report::review::ReviewState::Valid, // TODO: replace with actual state
    })
    .collect();

    let children = if children.is_empty() {
        None
    } else {
        Some(children)
    };
    let parents = if parents.is_empty() {
        None
    } else {
        Some(parents)
    };
    let covered_by = if covering_test_runs.is_empty() && covering_test_cases.is_empty() {
        None
    } else {
        Some(RequirementCoverageByTestsOverview {
            test_runs: covering_test_runs,
            test_cases: covering_test_cases,
        })
    };
    let reviewed_in = if reviewed_in.is_empty() {
        None
    } else {
        Some(reviewed_in)
    };

    Ok(RequirementEvidence {
        id: req.id,
        title: req.title,
        state: req.state.try_into()?,
        optional: req.optional,
        manual_verification: req.manual_verification,
        parents,
        children,
        covered_by,
        reviewed_in,
    })
}
