use anyhow::Context;

use crate::cmd::collect::{collection::Collection, products::collection::ProductCollection};

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn remove_outdated_requirement_parts(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        todo!();

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
