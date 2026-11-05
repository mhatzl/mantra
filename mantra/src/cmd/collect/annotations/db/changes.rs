use anyhow::Context;

use crate::cmd::collect::{collection::Collection, products::collection::ProductCollection};

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn remove_outdated_annotation_parts(&mut self) -> Result<(), anyhow::Error> {
        let collection_id = self.collect_nr();
        let product_id = self.product_id().clone();

        sqlx::query!(
            "
            delete from ConfigTraceProperties
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ConfigTraceProperties")?;

        sqlx::query!(
            "
            delete from AnnotationCollectConfigs
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated AnnotationCollectConfigs")?;

        sqlx::query!(
            "
            delete from ProductAnnotationSources
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ProductAnnotationSources")?;

        Ok(())
    }
}

impl<'db> Collection<'db> {
    pub(crate) async fn reset_annotation_changes(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from AddedAnnotationSources
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting added annotation history")?;

        sqlx::query!(
            "
            delete from RemovedAnnotationSources
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting removed annotation history")?;

        Ok(())
    }
}
