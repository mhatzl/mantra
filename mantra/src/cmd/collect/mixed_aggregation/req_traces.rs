use anyhow::Context;

use crate::cmd::collect::products::collection::ProductCollection;

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn update_direct_req_traces(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();

        // "on conflict update" at the end because PK may remain unchanged for new collections,
        // but we want "agg_collect_nr" to be the latest that updated an entry.
        sqlx::query!(
            "
            insert into DirectProductReqTraces (
                agg_collect_nr,
                product_id,
                req_collect_nr,
                req_id,
                file_collect_nr,
                filepath,
                file_hash,
                line
            )
            select
                $1 as agg_collect_nr,
                $2 as product_id,
                r.collect_nr as req_collect_nr,
                rt.req_id,
                pas.collect_nr as file_collect_nr,
                pas.filepath,
                rt.file_hash,
                rt.line
            from DetectedReqTraces rt, Requirements r, ProductAnnotationSources pas
            where rt.last_collect_nr >= pas.collect_nr
            and pas.collect_nr = (
                select max(cpas.collect_nr)
                from ProductAnnotationSources cpas
                where cpas.product_id = $2
            )
            and r.collect_nr = (
                select max(cr.collect_nr)
                from Requirements cr
                where cr.product_id = $2
            )
            and r.product_id = $2 and pas.product_id = $2
            and rt.file_hash = pas.file_hash
            and r.id = rt.req_id

            union all

            select
                $1 as agg_collect_nr,
                $2 as product_id,
                r.collect_nr as req_collect_nr,
                rt.req_id,
                pas.collect_nr as file_collect_nr,
                pas.filepath,
                rt.file_hash,
                rt.line
            from DetectedProductReqTraces rt, Requirements r, ProductAnnotationSources pas
            where rt.last_collect_nr >= pas.collect_nr
            and pas.collect_nr = (
                select max(cpas.collect_nr)
                from ProductAnnotationSources cpas
                where cpas.product_id = $2
            )
            and r.collect_nr = (
                select max(cr.collect_nr)
                from Requirements cr
                where cr.product_id = $2
            )
            and r.product_id = $2 and pas.product_id = $2
            and rt.product_id = $2
            and rt.file_hash = pas.file_hash
            and r.id = rt.req_id

            on conflict (
                product_id,
                req_collect_nr,
                req_id,
                file_collect_nr,
                filepath,
                line
            ) do update
            set
                agg_collect_nr = excluded.agg_collect_nr,
                file_hash = excluded.file_hash
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Updating product traces")?;

        Ok(())
    }
}
