use anyhow::Context;
use mantra_schema::{FmtHash, product::Product};

use crate::cmd::collect::products::collection::ProductCollection;

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn collect_product_metadata(
        &mut self,
        product: &Product,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = &product.id;

        let data_hash = FmtHash::new_serialized(product);

        let description_hash = if let Some(description) = &product.description {
            let hash = self
                .insert_general_text(description)
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
                latest_collection,
                id,
                name,
                base,
                version,
                homepage,
                repository,
                license,
                description_hash,
                data_hash
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
            on conflict (
                id
            ) do update
            set
                latest_collection = excluded.latest_collection,
                name = excluded.name,
                base = excluded.base,
                version = excluded.version,
                homepage = excluded.homepage,
                repository = excluded.repository,
                license = excluded.license,
                description_hash = excluded.description_hash,
                data_hash = excluded.data_hash
            ",
            collect_nr,
            product_id,
            product.name,
            product.base,
            product.version,
            product.homepage,
            product.repository,
            product.license,
            description_hash,
            data_hash
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to insert the product base data")?;

        if let Some(properties) = &product.properties {
            for property in properties {
                let key = property.0;
                let value = property.1;
                let hash = self.insert_general_json(value).await.with_context(|| {
                    format!("Failed to insert the content for property '{}'", key)
                })?;

                sqlx::query!(
                    "
                    insert into ProductProperties (
                        latest_collection,
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
                    on conflict (
                        product_id,
                        property_key
                    ) do update
                    set
                        latest_collection = excluded.latest_collection,
                        value_hash = excluded.value_hash
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

        if let Some(media_type) = &product.media_type
            && let Some(mimes) = media_type.mimes()
        {
            let mut index = 0;

            for mime in mimes {
                let raw_mime = mime.to_string();
                index += 1; // increase before for 1-indexed entries

                sqlx::query!(
                    "
                    insert into ProductMetadataMediaTypes (
                        latest_collection,
                        product_id,
                        media_type,
                        entry_number
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4
                    )
                    on conflict (
                        product_id,
                        media_type
                    ) do update
                    set
                        latest_collection = excluded.latest_collection,
                        entry_number = excluded.entry_number
                    ",
                    collect_nr,
                    product.id,
                    raw_mime,
                    index
                )
                .execute(self.connection_mut())
                .await
                .context("Failed to insert product metadata media types")?;
            }
        }

        Ok(())
    }
}
