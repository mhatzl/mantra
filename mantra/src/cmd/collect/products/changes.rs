use anyhow::Context;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn remove_outdated_products(&mut self) -> Result<(), anyhow::Error> {
        let collection_id = self.collect_nr();

        sqlx::query!(
            "
            delete from ProductProperties
            where latest_collection < $1
            ",
            collection_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ProductProperties")?;

        sqlx::query!(
            "
            delete from ProductRelatedHashedFiles
            where latest_collection < $1
            ",
            collection_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ProductRelatedHashedFiles")?;

        sqlx::query!(
            "
            delete from ProductRelatedFiles
            where latest_collection < $1
            ",
            collection_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ProductRelatedFiles")?;

        sqlx::query!(
            "
            delete from ProductMetadataMediaTypes
            where latest_collection < $1
            ",
            collection_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ProductMetadataMediaTypes")?;

        sqlx::query!(
            "
            delete from Products
            where latest_collection < $1
            ",
            collection_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated products")?;

        Ok(())
    }

    pub(crate) async fn reset_product_changes(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from AddedProducts
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting added product history")?;

        sqlx::query!(
            "
            delete from RemovedProducts
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting removed product history")?;

        sqlx::query!(
            "
            delete from ChangedProducts
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting changed product history")?;

        Ok(())
    }
}
