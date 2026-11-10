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
            select max(latest_collection) as "latest_collection:i64"
            from Requirements
            "#
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying latest collection updating requirements")?
        .and_then(|r| r.latest_collection);

        let latest_collected_annotations_nr = sqlx::query!(
            r#"
            select max(latest_collection) as "latest_collection:i64"
            from ProductAnnotationSources
            "#
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying latest collection updating annotations")?
        .and_then(|r| r.latest_collection);

        let latest_collected_review_nr = sqlx::query!(
            r#"
            select max(latest_collection) as "latest_collection:i64"
            from Reviews
            "#
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying latest collection updating reviews")?
        .and_then(|r| r.latest_collection);

        let requirements_collected = latest_collected_requirements_nr == Some(collect_nr);
        let annotations_collected = latest_collected_annotations_nr == Some(collect_nr);
        let reviews_collected = latest_collected_review_nr == Some(collect_nr);

        // Note: Order is important, because later queries build on updated tables

        if requirements_collected || annotations_collected || reviews_collected {
            self.update_directly_satisfied_requirements().await?;
        }

        self.update_obsoletions()
            .await
            .context("Updating obsolete test runs and reviews")?;

        self.resolve_tests()
            .await
            .context("Resolving test states")?;

        self.aggregate_trace_mappings()
            .await
            .context("Updating trace mappings")?;

        self.update_verification_states()
            .await
            .context("Updating requirement verification states")?;

        Ok(())
    }

    pub(crate) async fn reset_aggregated_changes(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from AddedLineCoverage
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting added line coverage history")?;

        sqlx::query!(
            "
            delete from RemovedLineCoverage
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting removed line coverage history")?;

        sqlx::query!(
            "
            delete from ChangedLineCoverage
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting changed line coverage history")?;

        sqlx::query!(
            "
            delete from ChangedRequirementVerificationStates
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting requirement verification state history")?;

        Ok(())
    }
}
