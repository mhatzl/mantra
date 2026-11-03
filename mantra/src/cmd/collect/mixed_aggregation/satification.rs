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
            product_id,
            req_id
        )
        with NonManualSatisfyTraced (product_id, req_id) as (
            select ur.product_id, ur.req_id
            from UsableNonManualRequirements ur, DirectProductReqTraces dt, Traces t
            where ur.product_id = dt.product_id
            and ur.req_id = dt.req_id and dt.file_hash = t.file_hash
            and dt.line = t.line
            and t.kind = $1
        ),
        ManualReviewed (product_id, req_id) as (
            select mr.product_id, mr.req_id
            from UsableManualRequirements mr, ManuallyVerifiedRequirements vr
            where mr.product_id = vr.product_id
            and mr.req_id = vr.req_id
        )

        select product_id, req_id
        from NonManualSatisfyTraced

        union

        select product_id, req_id
        from ManualReviewed
        ",
            satisfies_kind
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
