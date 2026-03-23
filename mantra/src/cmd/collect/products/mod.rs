use mantra_schema::{FmtHash, product::Product};

use crate::cmd::collect::Collection;

#[cfg(test)]
mod tests;

impl<'db> Collection<'db> {
    pub(super) async fn update_product(&mut self, product: Product) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = product.id;

        let description_hash = if let Some(description) = &product.description {
            let hash = FmtHash::from(&description);
            self.insert_general_text(&hash, description.clone(), None)
                .await?;
            Some(hash)
        } else {
            None
        };

        // Note: Only one product per collection possible, so duplicate definitions must be detected before.
        sqlx::query!(
            "
            insert into Products (
                last_collect_nr,
                id,
                name,
                base,
                version,
                homepage,
                repository,
                license,
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
                $9
            )
            on conflict (id)
            do update set
                last_collect_nr = excluded.last_collect_nr,
                name = excluded.name,
                base = excluded.base,
                version = excluded.version,
                homepage = excluded.homepage,
                repository = excluded.repository,
                license = excluded.license,
                description_hash = excluded.description_hash
            ",
            collect_nr,
            product_id,
            product.name,
            product.base,
            product.version,
            product.homepage,
            product.repository,
            product.license,
            description_hash
        )
        .execute(self.connection_mut())
        .await?;

        if let Some(properties) = product.properties {
            for property in properties {
                let key = property.0;
                let value = property.1;
                let hash = FmtHash::from(&value);

                self.insert_general_json(&hash, value).await?;

                sqlx::query!(
                    "
                    insert into ProductProperties (
                        last_collect_nr,
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
                    on conflict (product_id, property_key)
                    do update set
                        last_collect_nr = excluded.last_collect_nr,
                        value_hash = excluded.value_hash
                    ",
                    collect_nr,
                    product_id,
                    key,
                    hash
                )
                .execute(self.connection_mut())
                .await?;
            }
        }

        Ok(())
    }

    pub(crate) async fn delete_outdated_product_info(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            delete from ProductProperties
            where product_id = $1 and last_collect_nr < $2
            ",
            product_id,
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
                delete from ProductRelatedFiles
                where product_id = $1 and last_collect_nr < $2
            ",
            product_id,
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
