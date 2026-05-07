use mantra_schema::{
    SCHEMA_VERSION,
    report::{
        product::ProductReportSchema,
        requirement::RequirementReference,
        requirements::{RequirementsReportSchema, RequirementsSummary},
    },
};
use toml::de;

use crate::db::MantraTransaction;

pub async fn generate_requirements_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
) -> Result<RequirementsReportSchema, anyhow::Error> {
    let requirements = sqlx::query!(
        r#"
        select
            rs.product_id,
            rs.id,
            rs.state,
            case when exists (
                select o.id
                from OptionalRequirements o
                where o.product_id = $1 and o.id = rs.id
            ) then true
            else false
            end as "optional!:bool"
        from RequirementVerificationStates rs
        where rs.product_id = $1
        "#,
        product.id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let metrics = sqlx::query!(
        r#"
        with Mandatory(id) as (
            select r.id
            from Requirements r
            where r.product_id = $1 and not exists (
                select o.id
                from OptionalRequirements o
                where o.product_id = $1 and o.id = r.id
            )
        ),
        Manual(id) as (
            select mr.id
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
                count(id) as mandatory_total,
                0 as mandatory_verified,
                0 as manuals_total,
                0 as manuals_verified
            from Mandatory

            union all

            select
                0 as mandatory_total,
                count(m.id) as mandatory_verified,
                0 as manuals_total,
                0 as manuals_verified
            from Mandatory m
            where exists (
                select vr.id
                from VerifiedRequirements vr
                where vr.product_id = $1 and vr.id = m.id
            )

            union all

            select
                0 as mandatory_total,
                0 as mandatory_verified,
                count(mr.id) as manuals_total,
                0 as manuals_verified
            from Manual mr

            union all

            select
                0 as mandatory_total,
                0 as mandatory_verified,
                0 as manuals_total,
                count(mr.id) as manuals_verified
            from Manual mr
            where exists (
                select vr.id
                from VerifiedRequirements vr
                where vr.product_id = $1 and vr.id = mr.id
            )
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
    let mut unverified = Vec::new();
    let mut verified = Vec::new();
    let mut ignored = Vec::new();
    let mut deprecated = Vec::new();

    for req in requirements {
        let reference = RequirementReference {
            url_part: req.id.replace('.', "/"),
            product_id: req.product_id,
            id: req.id,
            state: req.state.try_into()?,
            optional: req.optional,
        };

        match reference.state {
            mantra_schema::report::requirement::RequirementState::Failed => failed.push(reference),
            mantra_schema::report::requirement::RequirementState::Verified => {
                verified.push(reference)
            }
            mantra_schema::report::requirement::RequirementState::Skipped => {
                skipped.push(reference)
            }
            mantra_schema::report::requirement::RequirementState::Unverified => {
                unverified.push(reference)
            }
            mantra_schema::report::requirement::RequirementState::Deprecated => {
                deprecated.push(reference)
            }
            mantra_schema::report::requirement::RequirementState::Ignored => {
                ignored.push(reference)
            }
        }
    }

    summary.failed.cnt = failed.len() as i64;
    summary.verified.cnt = verified.len() as i64;
    summary.skipped.cnt = skipped.len() as i64;
    summary.unverified.cnt = unverified.len() as i64;
    summary.deprecated.cnt = deprecated.len() as i64;
    summary.ignored.cnt = ignored.len() as i64;

    summary.update_percentages();

    Ok(RequirementsReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        summary,
        failed,
        skipped,
        unverified,
        verified,
        ignored,
        deprecated,
    })
}
