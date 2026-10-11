pub mod obsoletion;
pub mod req_traces;
pub mod satification;
pub mod test_states;
pub mod trace_mapping;
pub mod verification;

use anyhow::{Context, anyhow};

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

        let latest_collected_test_nr = sqlx::query!(
            r#"
            select max(collect_nr) as "latest_collect_nr:i64"
            from TestRuns
            "#
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying latest collection updating tests")?
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
        let tests_collected = latest_collected_test_nr == Some(collect_nr);
        let reviews_collected = latest_collected_review_nr == Some(collect_nr);

        // Note: Order is important, because later queries build on updated tables

        if requirements_collected || annotations_collected || reviews_collected {
            self.update_directly_satisfied_requirements().await?;
        }

        self.update_obsoletions()
            .await
            .context("Updating obsolete test runs and reviews")?;

        if annotations_collected || tests_collected || reviews_collected {
            self.resolve_tests()
                .await
                .context("Resolving test states")?;
        }

        
        
        Ok(())
    }
}
