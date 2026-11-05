use anyhow::Context;

use crate::cmd::collect::{collection::Collection, products::collection::ProductCollection};

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn remove_outdated_requirement_parts(&mut self) -> Result<(), anyhow::Error> {
        let collection_id = self.collect_nr();
        let product_id = self.product_id().clone();

        sqlx::query!(
            "
            delete from RequirementReplacements
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated RequirementReplacements")?;

        sqlx::query!(
            "
            delete from RequirementChildren
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated RequirementChildren")?;

        sqlx::query!(
            "
            delete from RequirementParents
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated RequirementParents")?;

        sqlx::query!(
            "
            delete from RequirementProperties
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated RequirementProperties")?;

        sqlx::query!(
            "
            delete from Requirements
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated Requirements")?;

        sqlx::query!(
            "
            delete from ConfigRequirementProperties
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated ConfigRequirementProperties")?;

        sqlx::query!(
            "
            delete from RequirementCollectConfigs
            where latest_collection < $1
            and product_id = $2
            ",
            collection_id,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting outdated RequirementCollectConfigs")?;

        Ok(())
    }
}

impl<'db> Collection<'db> {
    pub(crate) async fn reset_requirement_changes(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from AddedRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting added requirement history")?;

        sqlx::query!(
            "
            delete from RemovedRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting removed requirement history")?;

        sqlx::query!(
            "
            delete from ChangedRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Deleting changed requirement history")?;

        Ok(())
    }
}
