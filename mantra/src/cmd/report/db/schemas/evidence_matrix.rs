use mantra_schema::{
    SCHEMA_VERSION,
    product::ProductId,
    report::{
        evidence_matrix::{EvidenceMatrixSchema, RequirementEvidence},
        formattable_content::FormattableContent,
        product::ProductReportSchema,
        requirement::RequirementReference,
    },
    requirements::ReqId,
};

use crate::{
    cmd::report::db::schemas::requirement::{
        requirement_child_references, requirement_content_media_type, requirement_coverage,
        requirement_parent_references, requirement_reviews,
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
            exists (
                select o.req_id
                from OptionalRequirements o
                where o.product_id = $1 and o.req_id = r.id
            ) as "optional!:bool",
            exists (
                select m.req_id
                from ManualRequirements m
                where m.product_id = $1 and m.req_id = r.id
            ) as "manual!:bool"
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
    manual: bool,
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

    let req_media_type = requirement_content_media_type(transaction, product_id, &req_id).await?;

    Ok(RequirementEvidence {
        req_ref: RequirementReference {
            product_id: product_id.clone(),
            id: req_id,
            state: req.state.try_into()?,
            optional: req.optional,
            manual: req.manual,
        },
        title: FormattableContent::new(req.title, req_media_type),
        parents,
        children,
        covered_by,
        reviewed_in,
    })
}
