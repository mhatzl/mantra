pub mod obsoletion;
pub mod req_traces;
pub mod satification;
pub mod test_states;
pub mod trace_mapping;
pub mod verification;

use anyhow::Context;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    /// Aggregates data that depends on collected data from more than one artifact category.
    pub(crate) async fn aggregate_mixed(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        let latest_collected_requirements_nr = sqlx::query!(
            r#"
            select max(collect_nr) as "latest_collect_nr:i64"
            from Requirements
            "#
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying latest collection updating requirements")?
        .and_then(|r| r.latest_collect_nr);

        let latest_collected_annotations_nr = sqlx::query!(
            r#"
            select max(collect_nr) as "latest_collect_nr:i64"
            from ProductAnnotationSources
            "#
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying latest collection updating annotations")?
        .and_then(|r| r.latest_collect_nr);

        let latest_collected_review_nr = sqlx::query!(
            r#"
            select max(collect_nr) as "latest_collect_nr:i64"
            from Reviews
            "#
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying latest collection updating reviews")?
        .and_then(|r| r.latest_collect_nr);

        let requirements_collected = latest_collected_requirements_nr == Some(collect_nr);
        let annotations_collected = latest_collected_annotations_nr == Some(collect_nr);
        let reviews_collected = latest_collected_review_nr == Some(collect_nr);

        // Note: Order is important, because later queries build on updated tables

        if requirements_collected || annotations_collected || reviews_collected {
            self.update_directly_satisfied_requirements().await?;
        }

        self.update_collected_file_hashes()
            .await
            .context("Updating collected file hashes")?;

        self.update_obsoletions()
            .await
            .context("Updating obsolete test runs and reviews")?;

        self.resolve_tests()
            .await
            .context("Resolving test states")?;

        self.update_verification_states()
            .await
            .context("Updating requirement verification states")?;

        Ok(())
    }

    async fn update_collected_file_hashes(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            with LatestHashes (collect_nr, filepath, file_hash) as (
                select
                    cf.collect_nr,
                    cf.filepath,
                    case
                        when cf.file_hash is null
                        -- may not lead to a hash value in case none got collected
                        then (
                            select cfi.file_hash
                            from CollectedFiles cfi
                            where cfi.filepath = cf.filepath
                            and cfi.file_hash is not null
                            and cfi.collect_nr < cf.collect_nr
                            order by cfi.collect_nr desc
                            limit 1
                        )
                        else cf.file_hash
                    end
                from CollectedFiles cf
                where cf.collect_nr = $1
            )
            insert or ignore into CollectedFileHashes (
                agg_collect_nr,
                filepath,
                file_hash
            )
            select
                collect_nr,
                filepath,
                file_hash
            from LatestHashes
            where file_hash is not null
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
