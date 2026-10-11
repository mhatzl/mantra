use anyhow::{Context, anyhow};
use mantra_schema::annotations::TraceKind;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(super) async fn update_directly_satisfied_requirements(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let satisfies_kind = TraceKind::Satisfies.as_nr();

        sqlx::query!(
            "
        insert or ignore into DirectlySatisfiedRequirements (
            agg_collect_nr,
            req_collect_nr,
            product_id,
            req_id
        )
        with NonManualSatisfyTraced (req_collect_nr, product_id, req_id) as (
            select ur.req_collect_nr, ur.product_id, ur.req_id
            from UsableNonManualRequirements ur, DirectProductReqTraces dt, Traces t
            where ur.agg_collect_nr = (
                select max(cur.agg_collect_nr)
                from UsableNonManualRequirements cur
            )
            and dt.agg_collect_nr = (
                select max(cdt.agg_collect_nr)
                from DirectProductReqTraces cdt
                where cdt.product_id = dt.product_id
            )
            and ur.req_collect_nr = dt.req_collect_nr
            and ur.product_id = dt.product_id
            and ur.req_id = dt.req_id and dt.file_hash = t.file_hash
            and dt.line = t.line
            and t.kind = $2
        ),
        ManualReviewed (req_collect_nr, product_id, req_id) as (
            select mr.req_collect_nr, mr.product_id, mr.req_id
            from UsableManualRequirements mr, ManuallyVerifiedRequirements vr
            where mr.agg_collect_nr = (
                select max(cmr.agg_collect_nr)
                from UsableManualRequirements cmr
            )
            and vr.collect_nr = (
                select max(crv.collect_nr)
                from Reviews crv
                where crv.product_id = vr.product_id
            )
            and mr.req_collect_nr = vr.req_collect_nr
            and mr.product_id = vr.product_id
            and mr.req_id = vr.req_id
        )

        select $1 as agg_collect_nr, req_collect_nr, product_id, req_id
        from NonManualSatisfyTraced

        union

        select $1 as agg_collect_nr, req_collect_nr, product_id, req_id
        from ManualReviewed
        ",
            collect_nr,
            satisfies_kind
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
