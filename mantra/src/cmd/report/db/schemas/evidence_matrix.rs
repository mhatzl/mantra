use mantra_schema::{
    SCHEMA_VERSION,
    product::ProductId,
    report::{
        evidence_matrix::{
            EvidenceMatrixSchema, RequirementCoverageByTestsOverview, RequirementEvidence,
        },
        product::ProductReportSchema,
        review::ReviewReference,
    },
    requirements::ReqId,
};

use crate::{
    cmd::report::db::schemas::requirement::{
        covering_test_cases, covering_test_runs, requirement_child_references,
        requirement_parent_references,
    },
    db::MantraTransaction,
};

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
    let children = requirement_child_references(transaction, product_id, &req.id).await?;
    let parents = requirement_parent_references(transaction, product_id, &req.id).await?;

    let covering_test_runs = covering_test_runs(transaction, product_id, &req.id).await?;
    let covering_test_cases = covering_test_cases(transaction, product_id, &req.id).await?;

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
