use std::path::PathBuf;

use anyhow::Context;
use mantra_schema::{
    FmtHash,
    path::RelativePath,
    product::{Product, ProductId},
};

use crate::{cmd::collect::collection::Collection, db::MantraConnection};

pub(super) struct ProductCollection<'db, 'c> {
    collection: &'c mut Collection<'db>,
    product_id: ProductId,
}

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(super) async fn new(
        collection: &'c mut Collection<'db>,
        product: &Product,
    ) -> Result<Self, anyhow::Error> {
        let mut pc = Self {
            collection,
            product_id: product.id.clone(),
        };

        pc.collect_product(product)
            .await
            .context("Failed collecting product metadata")?;

        Ok(pc)
    }

    pub(super) fn connection_mut(&mut self) -> &mut MantraConnection {
        self.collection.connection_mut()
    }

    pub(super) fn collect_nr(&self) -> i64 {
        self.collection.collect_nr()
    }

    pub(super) fn product_id(&self) -> &ProductId {
        &self.product_id
    }

    pub(super) async fn insert_general_json(
        &mut self,
        hash: &FmtHash,
        content: &serde_json::Value,
    ) -> Result<(), anyhow::Error> {
        self.collection.insert_general_json(hash, content).await
    }

    pub(super) async fn insert_general_text(
        &mut self,
        hash: &FmtHash,
        content: &str,
    ) -> Result<(), anyhow::Error> {
        self.collection.insert_general_text(hash, content).await
    }

    pub(super) async fn collect_unhashed_file(
        &mut self,
        filepath: &RelativePath,
    ) -> Result<(), anyhow::Error> {
        self.collection.collect_unhashed_file(filepath).await
    }

    pub(super) async fn insert_collected_file(
        &mut self,
        filepath: &RelativePath,
        file_hash: &FmtHash,
        content: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        self.collection
            .insert_collected_file(filepath, file_hash, content)
            .await?;

        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let filepath = filepath.as_str();

        sqlx::query!(
            "
            insert or ignore into ProductRelatedFiles (
                collect_nr,
                product_id,
                filepath
            )
            values (
                $1,
                $2,
                $3
            )
            ",
            collect_nr,
            product_id,
            filepath
        )
        .execute(self.connection_mut())
        .await
        .with_context(|| format!("Failed inserting product related file: {}", filepath))?;

        Ok(())
    }

    /// Returns the absolute path to the directory the used mantra config file is located in.
    pub(super) fn abs_cfg_file_parent_path(&self) -> PathBuf {
        self.collection.abs_cfg_file_parent_path()
    }
}
