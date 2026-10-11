use anyhow::Context;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(super) async fn update_obsoletions(&mut self) -> Result<(), anyhow::Error> {
        self.update_obsolete_test_runs()
            .await
            .context("Failed to update obsolete test runs")?;
        self.update_valid_test_runs()
            .await
            .context("Updating valid test runs")?;

        self.update_obsolete_reviews()
            .await
            .context("Failed to update obsolete reviews")?;
        self.update_valid_reviews()
            .await
            .context("Updating valid reviews")?;

        Ok(())
    }

    async fn update_obsolete_test_runs(&mut self) -> Result<(), anyhow::Error> {
        // TODO: Adapt collect cmd to collect reqs, annotations, tests, and reviews separately.
        // If reqs and annotations for the same product are collected in a later collection,
        // tests are marked as obsolete (same for reviews).

        sqlx::query!(
            "
            insert or ignore into ObsoleteTestRuns (
                collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            select collect_nr, product_id, name, utc_date
            from TestRuns tr
            where
            -- requirements collected since test was added
            tr.collect_nr < (
                select max(r.collect_nr)
                from Requirements r
                where r.product_id = tr.product_id
            )
            -- annotations collected since test was added
            or tr.collect_nr < (
                select max(a.collect_nr)
                from ProductAnnotationSources a
                where a.product_id = tr.product_id
            )
            -- test run with same name has been collected in later collection
            or tr.collect_nr < (
                select max(ctr.collect_nr)
                from TestRuns ctr
                where ctr.product_id = tr.product_id
                and ctr.name = tr.name
            )
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_valid_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into ValidTestRuns (
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            select $1 as agg_collect_nr, collect_nr, product_id, name, utc_date
            from TestRuns tr
            where not exists (
                select otr.test_run_name
                from ObsoleteTestRuns otr
                where otr.collect_nr = tr.collect_nr
                and otr.product_id = tr.product_id
                and otr.test_run_name = tr.name
                and otr.test_run_date = tr.utc_date
            )
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_obsolete_reviews(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            with TestLineCoverageOverrides (collect_nr, product_id, review_name, review_date, test_collect_nr, test_run_name, test_run_date) as (
                select collect_nr, product_id, review_name, review_date, test_collect_nr, test_run_name, test_run_date
                from TestRunLineCoverageOverrides

                union all

                select collect_nr, product_id, review_name, review_date, test_collect_nr, test_run_name, test_run_date
                from TestCaseLineCoverageOverrides
            ), TestOverrides (collect_nr, product_id, review_name, review_date, test_collect_nr, test_run_name, test_run_date) as (
                select collect_nr, product_id, review_name, review_date, test_collect_nr, test_run_name, test_run_date
                from TestCaseOverrides

                union all

                select collect_nr, product_id, review_name, review_date, test_collect_nr, test_run_name, test_run_date
                from TestLineCoverageOverrides
            )
            insert or ignore into ObsoleteReviews (
                collect_nr,
                product_id,
                review_name,
                review_date
            )
            select collect_nr, product_id, name, utc_date
            from Reviews rv
            where
            -- requirements collected since review was added
            rv.collect_nr < (
                select max(r.collect_nr)
                from Requirements r
                where r.product_id = rv.product_id
            )
            -- review with same name has been collected in later collection
            or rv.collect_nr < (
                select max(crv.collect_nr)
                from Reviews crv
                where crv.product_id = rv.product_id
                and crv.name = rv.name
            )

            union all

            select o.collect_nr, o.product_id, o.review_name, o.review_date
            from TestOverrides o
            where o.test_collect_nr < (
                select max(tr.collect_nr)
                from TestRuns tr
                where tr.product_id = o.product_id
                and tr.name = o.test_run_name
            )
            or exists (
                select otr.test_run_name
                from ObsoleteTestRuns otr
                where otr.collect_nr = o.test_collect_nr
                and otr.product_id = o.product_id
                and otr.test_run_name = o.test_run_name
                and otr.test_run_date = o.test_run_date
            )
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_valid_reviews(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into ValidReviews (
                agg_collect_nr,
                review_collect_nr,
                product_id,
                review_name,
                review_date
            )
            select $1 as agg_collect_nr, collect_nr, product_id, name, utc_date
            from Reviews rv
            where not exists (
                select orv.review_name
                from ObsoleteReviews orv
                where orv.collect_nr = rv.collect_nr
                and orv.product_id = rv.product_id
                and orv.review_name = rv.name
                and orv.review_date = rv.utc_date
            )
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
