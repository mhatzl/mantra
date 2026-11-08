use mantra_schema::{
    SCHEMA_VERSION,
    product::ProductId,
    report::{
        evidence_matrix::{EvidenceMatrixSchema, RequirementEvidence},
        product::ProductReportSchema,
    },
    requirements::ReqId,
};

use crate::{
    cmd::report::db::schemas::requirement::{
        covering_test_cases, covering_test_runs, requirement_child_references,
        requirement_coverage, requirement_parent_references, requirement_reviews,
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
                select o.req_id
                from OptionalRequirements o
                where o.product_id = $1 and o.req_id = r.id
            ) then true
            else false
            end as "optional!:bool",
            case when exists (
                select m.req_id
                from ManualRequirements m
                where m.product_id = $1 and m.req_id = r.id
            ) then true
            else false
            end as "manual_verification!:bool"
        from Requirements r, RequirementVerificationStates rs
        where r.product_id = $1 and rs.product_id = $1
        and r.id = rs.req_id
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
    id: String,
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
    let req_id = ReqId::new(req.id)?;

    let children = requirement_child_references(transaction, product_id, &req_id).await?;
    let parents = requirement_parent_references(transaction, product_id, &req_id).await?;

    let covered_by = requirement_coverage(transaction, product_id, &req_id).await?;
    let reviewed_in = requirement_reviews(transaction, product_id, &req_id).await?;

    Ok(RequirementEvidence {
        product_id: product_id.clone(),
        id: req_id,
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
