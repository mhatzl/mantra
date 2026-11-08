use mantra_schema::{
    SCHEMA_VERSION,
    report::{
        product::ProductReportSchema,
        requirement::RequirementReference,
        requirements::{RequirementsOverview, RequirementsReportSchema, RequirementsSummary},
    },
};

use crate::db::MantraTransaction;

pub async fn generate_requirements_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
) -> Result<RequirementsReportSchema, anyhow::Error> {
    let requirements = sqlx::query!(
        r#"
        select
            rs.product_id,
            rs.req_id,
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
        from RequirementVerificationStates rs
        where rs.product_id = $1
        "#,
        product.id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let metrics = sqlx::query!(
        r#"
        with Mandatory(req_id) as (
            select r.id
            from Requirements r
            where r.product_id = $1 and not exists (
                select o.req_id
                from OptionalRequirements o
                where o.product_id = $1 and o.req_id = r.id
            )
        ),
        Manual(req_id) as (
            select mr.req_id
            from ManualRequirements mr
            where mr.product_id = $1
        )
        select
            sum(mandatory_total) as "mandatory_total!:i64",
            sum(mandatory_verified) as "mandatory_verified!:i64",
            sum(manuals_total) as "manuals_total!:i64",
            sum(manuals_verified) as "manuals_verified!:i64"
        from (
            select
                count(req_id) as mandatory_total,
                0 as mandatory_verified,
                0 as manuals_total,
                0 as manuals_verified
            from Mandatory

            union all

            select
                0 as mandatory_total,
                count(m.req_id) as mandatory_verified,
                0 as manuals_total,
                0 as manuals_verified
            from Mandatory m, VerifiedRequirements vr
            where vr.product_id = $1 and vr.req_id = m.req_id


            union all

            select
                0 as mandatory_total,
                0 as mandatory_verified,
                count(mr.req_id) as manuals_total,
                0 as manuals_verified
            from Manual mr

            union all

            select
                0 as mandatory_total,
                0 as mandatory_verified,
                0 as manuals_total,
                count(mr.req_id) as manuals_verified
            from Manual mr, VerifiedRequirements vr
            where vr.product_id = $1 and vr.req_id = mr.req_id
        )
        "#,
        product.id
    )
    .fetch_one(transaction.as_mut())
    .await?;

    let mut summary = RequirementsSummary {
        total: requirements.len() as i64,
        ..Default::default()
    };
    summary.mandatory_total.cnt = metrics.mandatory_total;
    summary.mandatory_verified.cnt = metrics.mandatory_verified;
    summary.manuals_total.cnt = metrics.manuals_total;
    summary.manuals_verified.cnt = metrics.manuals_verified;

    let mut failed = Vec::new();
    let mut skipped = Vec::new();
    let mut unknown = Vec::new();
    let mut satisfied = Vec::new();
    let mut verified = Vec::new();
    let mut partially_verified = Vec::new();
    let mut excluded = Vec::new();
    let mut deprecated = Vec::new();
    let mut draft = Vec::new();

    for req in requirements {
        let reference = RequirementReference {
            product_id: product.id.clone(),
            id: req.req_id.try_into()?,
            state: req.state.try_into()?,
            optional: req.optional,
            manual: req.manual,
        };

        match reference.state {
            mantra_schema::report::requirement::RequirementState::Failed => failed.push(reference),
            mantra_schema::report::requirement::RequirementState::Verified => {
                verified.push(reference)
            }
            mantra_schema::report::requirement::RequirementState::Skipped => {
                skipped.push(reference)
            }
            mantra_schema::report::requirement::RequirementState::Unknown => {
                unknown.push(reference)
            }
            mantra_schema::report::requirement::RequirementState::Deprecated => {
                deprecated.push(reference)
            }
            mantra_schema::report::requirement::RequirementState::Excluded => {
                excluded.push(reference)
            }
            mantra_schema::report::requirement::RequirementState::Satisfied => {
                satisfied.push(reference)
            }
            mantra_schema::report::requirement::RequirementState::PartiallyVerified => {
                partially_verified.push(reference)
            }
            mantra_schema::report::requirement::RequirementState::Draft => draft.push(reference),
        }
    }

    summary.failed.cnt = failed.len() as i64;
    summary.verified.cnt = verified.len() as i64;
    summary.skipped.cnt = skipped.len() as i64;
    summary.unknown.cnt = unknown.len() as i64;
    summary.satisfied.cnt = satisfied.len() as i64;
    summary.partially_verified.cnt = partially_verified.len() as i64;
    summary.deprecated.cnt = deprecated.len() as i64;
    summary.excluded.cnt = excluded.len() as i64;
    summary.draft.cnt = draft.len() as i64;

    summary.update_percentages();

    Ok(RequirementsReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        requirements: RequirementsOverview {
            summary,
            failed,
            skipped,
            unknown,
            verified,
            excluded,
            deprecated,
            draft,
            satisfied,
            partially_verified,
        },
    })
}
