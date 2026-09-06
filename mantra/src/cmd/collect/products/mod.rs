use anyhow::Context;
use mantra_schema::{FmtHash, product::Product};

use crate::cmd::collect::product_collection::ProductCollection;

#[cfg(test)]
mod tests;

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(super) async fn collect_product(&mut self, product: &Product) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = &product.id;

        let description_hash = if let Some(description) = &product.description {
            let hash = FmtHash::from(&description);
            self.insert_general_text(&hash, description)
                .await
                .context("Failed to insert the product description")?;
            Some(hash)
        } else {
            None
        };

        // Note: Only one product per collection possible, so duplicate definitions must be detected before.
        sqlx::query!(
            "
            insert into Products (
                collect_nr,
                id,
                name,
                base,
                version,
                homepage,
                repository,
                license,
                media_type,
                description_hash
            )
            values (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9,
                $10
            )
            ",
            collect_nr,
            product_id,
            product.name,
            product.base,
            product.version,
            product.homepage,
            product.repository,
            product.license,
            product.media_type,
            description_hash
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to insert the product base data")?;

        if let Some(properties) = &product.properties {
            for property in properties {
                let key = property.0;
                let value = property.1;
                let hash = FmtHash::from(&value);

                self.insert_general_json(&hash, &value)
                    .await
                    .with_context(|| {
                        format!("Failed to insert the content for property '{}'", key)
                    })?;

                sqlx::query!(
                    "
                    insert into ProductProperties (
                        collect_nr,
                        product_id,
                        property_key,
                        value_hash
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4
                    )
                    ",
                    collect_nr,
                    product_id,
                    key,
                    hash
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| format!("Failed to insert property '{}'", key))?;
            }
        }

        Ok(())
    }
}
