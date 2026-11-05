use anyhow::Context;

use crate::cmd::collect::{collection::Collection, products::collection::ProductCollection};

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn remove_outdated_review_parts(&mut self) -> Result<(), anyhow::Error> {
        let collection_id = self.collect_nr();
        let product_id = self.product_id().clone();

        sqlx::query!(
            "
            delete from IgnoredReviewEntries
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated IgnoredReviewEntries")?;

        sqlx::query!(
            "
            delete from TestCaseLineCoverageOverrides
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestCaseLineCoverageOverrides")?;

        sqlx::query!(
            "
            delete from TestRunLineCoverageOverrides
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestRunLineCoverageOverrides")?;

        sqlx::query!(
            "
            delete from TestCaseOverrides
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated TestCaseOverrides")?;

        sqlx::query!(
            "
            delete from ManuallyVerifiedRequirements
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ManuallyVerifiedRequirements")?;

        sqlx::query!(
            "
            delete from ReviewRevisionAuthors
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ReviewRevisionAuthors")?;

        sqlx::query!(
            "
            delete from ReviewRevisions
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ReviewRevisions")?;

        sqlx::query!(
            "
            delete from ReviewProperties
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ReviewProperties")?;

        sqlx::query!(
            "
            delete from ReviewAuthors
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ReviewAuthors")?;

        sqlx::query!(
            "
            delete from Reviews
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated Reviews")?;

        sqlx::query!(
            "
            delete from ConfigReviewProperties
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ConfigReviewProperties")?;

        sqlx::query!(
            "
            delete from ReviewCollectConfigs
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ReviewCollectConfigs")?;

        Ok(())
    }
}

impl<'db> Collection<'db> {
    pub(crate) async fn reset_review_changes(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from AddedReviews
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting added review history")?;

        sqlx::query!(
            "
            delete from RemovedReviews
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting removed review history")?;

        sqlx::query!(
            "
            delete from ChangedReviews
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting changed review history")?;

        Ok(())
    }
}
