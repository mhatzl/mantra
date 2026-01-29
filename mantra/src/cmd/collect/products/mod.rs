use mantra_schema::product::{Product, ProductId};

use crate::{
    cmd::collect::db::CollectTransaction,
    db::{ContentHash, FmtHash, Hash},
};

#[cfg(test)]
mod test;

impl<'db> CollectTransaction<'db> {
    pub async fn update_products(
        &'db mut self,
        product: &Product,
    ) -> Result<(FmtHash, ProductId), anyhow::Error> {
        let section_hash = product.fmt_hash();
        let section_ref = &section_hash;

        let product_id = product_id(product);
        let ref_id = &product_id;

        let metadata_hash = product.metadata.hash();
        let fmt_details_hash = product_details_hash(product, metadata_hash.clone()).finalize();

        let fmt_metadata_hash = if let Some(metadata) = &product.metadata {
            let hash = metadata_hash.finalize();
            let hash_ref = &hash;

            sqlx::query!(
                "
                insert or ignore into GeneralJson (
                    hash,
                    data
                )
                values (
                    $1,
                    $2
                )
                ",
                hash_ref,
                metadata
            )
            .execute(&mut *self.connection())
            .await?;

            Some(hash)
        } else {
            None
        };

        sqlx::query!(
            "
            insert or ignore into CollectedSections (hash)
            values ($1)
            ",
            section_ref
        )
        .execute(&mut *self.connection())
        .await?;

        sqlx::query!(
            "
                insert or ignore into Products (id)
                values ($1)
            ",
            ref_id,
        )
        .execute(&mut *self.connection())
        .await?;

        sqlx::query!(
            "
                insert or ignore into ProductBaselines (
                    id,
                    base,
                    version
                )
                values (
                    $1,
                    $2,
                    $3
                )
                ",
            ref_id,
            product.base,
            product.version
        )
        .execute(&mut *self.connection())
        .await?;

        sqlx::query!(
            "
                insert or ignore into ProductDetails (
                    hash,
                    name,
                    homepage,
                    repository,
                    license,
                    metadata_hash
                )
                values (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6
                )
                ",
            fmt_details_hash,
            product.name,
            product.homepage,
            product.repository,
            product.license,
            fmt_metadata_hash
        )
        .execute(&mut *self.connection())
        .await?;

        sqlx::query!(
            "
                insert or ignore into ProductCollections (
                    section_hash,
                    product_id,
                    product_base,
                    product_details_hash
                )
                values (
                    $1,
                    $2,
                    $3,
                    $4
                )
                ",
            section_ref,
            ref_id,
            product.base,
            fmt_details_hash
        )
        .execute(&mut *self.connection())
        .await?;

        Ok((section_hash, product_id))
    }
}

fn product_id(product: &Product) -> ProductId {
    match &product.id {
        Some(id) => id.clone(),
        None => serde_json::json!({
            "name": product.name,
            "base": product.base
        })
        .hash()
        .finalize(),
    }
}

fn product_details_hash(product: &Product, metadata_hash: Hash) -> Hash {
    let mut hash = product.name.hash();
    hash.extend(product.homepage.hash());
    hash.extend(product.repository.hash());
    hash.extend(product.license.hash());
    hash.extend(metadata_hash);
    hash
}
