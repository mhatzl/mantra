use anyhow::Context;

use crate::cmd::collect::products::collection::ProductCollection;

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn update_direct_req_traces(&mut self) -> Result<(), anyhow::Error> {
        let product_id = self.product_id().clone();

        sqlx::query!(
            "
            delete from DirectProductReqTraces
            where product_id = $1
            ",
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing DirectProductReqTraces")?;

        sqlx::query!(
            "
            insert into DirectProductReqTraces (
                product_id,
                req_id,
                filepath,
                file_hash,
                line
            )
            select
                $1 as product_id,
                rt.req_id,
                prh.filepath,
                rt.file_hash,
                rt.line
            from DetectedReqTraces rt, Requirements r, ProductRelatedHashedFiles prh
            where r.product_id = $1 and prh.product_id = $1
            and rt.file_hash = prh.file_hash
            and r.id = rt.req_id

            union all

            select
                $1 as product_id,
                rt.req_id,
                prh.filepath,
                rt.file_hash,
                rt.line
            from DetectedProductReqTraces rt, Requirements r, ProductRelatedHashedFiles prh
            where r.product_id = $1 and prh.product_id = $1
            and rt.product_id = $1
            and rt.file_hash = prh.file_hash
            and r.id = rt.req_id
            ",
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Updating product traces")?;

        Ok(())
    }
}
