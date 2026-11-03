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
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into ObsoleteTestRuns (
                product_id,
                test_run_name
            )
            select product_id, name
            from TestRuns tr
            where
            -- covered file hashes in conflicting hashes
            exists (
                select trc.cov_file_hash
                from TestRunLineCoverage trc, ProductRelatedHashedFiles phf
                where trc.product_id = tr.product_id
                and trc.test_run_name = tr.name
                and trc.cov_file_hash not null
                and trc.product_id = phf.product_id
                and trc.cov_filepath = phf.filepath
                and trc.cov_file_hash != phf.file_hash
            ) or exists (
                select trc.cov_file_hash
                from TestCaseLineCoverage trc, ProductRelatedHashedFiles phf
                where trc.product_id = tr.product_id
                and trc.test_run_name = tr.name
                and trc.cov_file_hash not null
                and trc.product_id = phf.product_id
                and trc.cov_filepath = phf.filepath
                and trc.cov_file_hash != phf.file_hash
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
                product_id,
                test_run_name
            )
            select product_id, name
            from TestRuns

            except

            select product_id, test_run_name
            from ObsoleteTestRuns
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_obsolete_reviews(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            with TestLineCoverageOverrides (product_id, review_name, test_run_name) as (
                select product_id, review_name, test_run_name
                from TestRunLineCoverageOverrides

                union all

                select product_id, review_name, test_run_name
                from TestCaseLineCoverageOverrides
            ), TestOverrides (product_id, review_name, test_run_name) as (
                select product_id, review_name, test_run_name
                from TestCaseOverrides

                union all

                select product_id, review_name, test_run_name
                from TestLineCoverageOverrides
            )
            insert or ignore into ObsoleteReviews (
                product_id,
                review_name
            )
            select
                product_id,
                name
            from Reviews rv
            where
            -- contains ignored review entries
            exists (
                select ie.review_name
                from IgnoredReviewEntries ie
                where ie.product_id = rv.product_id
                and ie.review_name = rv.name
            )

            union all

            select
                o.product_id,
                o.review_name
            from TestOverrides o
            where exists (
                select otr.test_run_name
                from ObsoleteTestRuns otr
                where otr.product_id = o.product_id
                and otr.test_run_name = o.test_run_name
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
                product_id,
                review_name
            )
            select product_id, name
            from Reviews rv
            where not exists (
                select orv.review_name
                from ObsoleteReviews orv
                where orv.product_id = rv.product_id
                and orv.review_name = rv.name
            )
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
