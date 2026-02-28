// use mantra_schema::product::{Product, ProductId};

// use crate::cmd::collect::db::CollectTransaction;

// #[cfg(test)]
// mod test;

use mantra_schema::{FmtHash, product::Product};

use crate::cmd::collect::Collection;

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

// impl<'db> CollectTransaction<'db> {
//     pub async fn update_products(
//         &'db mut self,
//         product: &Product,
//     ) -> Result<(FmtHash, ProductId), anyhow::Error> {
//         let section_hash = product.fmt_hash();
//         let section_ref = &section_hash;

//         let product_id = product_id(product);
//         let ref_id = &product_id;

//         let metadata_hash = product.metadata.hash();
//         let fmt_details_hash = product_details_hash(product, metadata_hash.clone()).finalize();

//         let fmt_metadata_hash = if let Some(metadata) = &product.metadata {
//             let hash = metadata_hash.finalize();
//             let hash_ref = &hash;

//             sqlx::query!(
//                 "
//                 insert or ignore into GeneralJson (
//                     hash,
//                     data
//                 )
//                 values (
//                     $1,
//                     $2
//                 )
//                 ",
//                 hash_ref,
//                 metadata
//             )
//             .execute(self.connection())
//             .await?;

//             Some(hash)
//         } else {
//             None
//         };

//         sqlx::query!(
//             "
//             insert or ignore into CollectedSections (hash)
//             values ($1)
//             ",
//             section_ref
//         )
//         .execute(self.connection())
//         .await?;

//         sqlx::query!(
//             "
//                 insert or ignore into Products (id)
//                 values ($1)
//             ",
//             ref_id,
//         )
//         .execute(self.connection())
//         .await?;

//         sqlx::query!(
//             "
//                 insert or ignore into ProductBaselines (
//                     id,
//                     base,
//                     version
//                 )
//                 values (
//                     $1,
//                     $2,
//                     $3
//                 )
//                 ",
//             ref_id,
//             product.base,
//             product.version
//         )
//         .execute(self.connection())
//         .await?;

//         sqlx::query!(
//             "
//                 insert or ignore into ProductDetails (
//                     hash,
//                     name,
//                     homepage,
//                     repository,
//                     license,
//                     metadata_hash
//                 )
//                 values (
//                     $1,
//                     $2,
//                     $3,
//                     $4,
//                     $5,
//                     $6
//                 )
//                 ",
//             fmt_details_hash,
//             product.name,
//             product.homepage,
//             product.repository,
//             product.license,
//             fmt_metadata_hash
//         )
//         .execute(self.connection())
//         .await?;

//         sqlx::query!(
//             "
//                 insert or ignore into ProductCollections (
//                     section_hash,
//                     product_id,
//                     product_base,
//                     product_details_hash
//                 )
//                 values (
//                     $1,
//                     $2,
//                     $3,
//                     $4
//                 )
//                 ",
//             section_ref,
//             ref_id,
//             product.base,
//             fmt_details_hash
//         )
//         .execute(self.connection())
//         .await?;

//         Ok((section_hash, product_id))
//     }
// }

// fn product_id(product: &Product) -> ProductId {
//     match &product.id {
//         Some(id) => id.clone(),
//         None => serde_json::json!({
//             "name": product.name,
//             "base": product.base
//         })
//         .hash()
//         .finalize(),
//     }
// }

// fn product_details_hash(product: &Product, metadata_hash: Hash) -> Hash {
//     let mut hash = product.name.hash();
//     hash.extend(product.homepage.hash());
//     hash.extend(product.repository.hash());
//     hash.extend(product.license.hash());
//     hash.extend(metadata_hash);
//     hash
// }
