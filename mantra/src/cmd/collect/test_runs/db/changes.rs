use anyhow::Context;

use crate::cmd::collect::{collection::Collection, products::collection::ProductCollection};

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn remove_outdated_test_run_parts(&mut self) -> Result<(), anyhow::Error> {
        let collection_id = self.collect_nr();
        let product_id = self.product_id().clone();

        sqlx::query!(
            "
            delete from TestCaseLineCoverage
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestCaseLineCoverage")?;

        sqlx::query!(
            "
            delete from TestCaseStateProperties
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestCaseStateProperties")?;

        sqlx::query!(
            "
            delete from TestCaseLocations
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestCaseLocations")?;

        sqlx::query!(
            "
            delete from TestCaseLogs
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestCaseLogs")?;

        sqlx::query!(
            "
            delete from TestCaseProperties
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestCaseProperties")?;

        sqlx::query!(
            "
            delete from TestCaseVerifiedRequirements
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestCaseVerifiedRequirements")?;

        sqlx::query!(
            "
            delete from TestCases
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestCases")?;

        sqlx::query!(
            "
            delete from TestRunLineCoverage
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestRunLineCoverage")?;

        sqlx::query!(
            "
            delete from TestRunLogs
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestRunLogs")?;

        sqlx::query!(
            "
            delete from TestRunHierarchies
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestRunHierarchies")?;

        sqlx::query!(
            "
            delete from TestRunRevisionAuthors
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestRunRevisionAuthors")?;

        sqlx::query!(
            "
            delete from TestRunRevisions
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestRunRevisions")?;

        sqlx::query!(
            "
            delete from TestRunProperties
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestRunProperties")?;

        sqlx::query!(
            "
            delete from TestRuns
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestRuns")?;

        sqlx::query!(
            "
            delete from ConfigTestCaseProperties
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ConfigTestCaseProperties")?;

        sqlx::query!(
            "
            delete from ConfigTestRunProperties
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ConfigTestRunProperties")?;

        sqlx::query!(
            "
            delete from TestRunCollectConfigs
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestRunCollectConfigs")?;

        Ok(())
    }
}

impl<'db> Collection<'db> {
    pub(crate) async fn reset_test_run_changes(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from AddedTestRuns
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting added test run history")?;

        sqlx::query!(
            "
            delete from RemovedTestRuns
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting removed test run history")?;

        sqlx::query!(
            "
            delete from ChangedTestRuns
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting changed test run history")?;

        Ok(())
    }
}
