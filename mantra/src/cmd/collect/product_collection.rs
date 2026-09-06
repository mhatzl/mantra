use std::path::PathBuf;

use anyhow::Context;
use mantra_schema::{
    FmtHash, Origin,
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

    pub(super) async fn insert_schema_single_sources(
        &mut self,
        content_hash: &FmtHash,
        origin: &Option<Origin>,
        filepath: &RelativePath,
    ) -> Result<(), anyhow::Error> {
        self.collection
            .insert_schema_single_source(content_hash, origin, filepath)
            .await
    }

    pub(super) async fn insert_schema_multi_sources(
        &mut self,
        content_hash: &FmtHash,
        origin: &Option<Origin>,
        filepaths: &[&RelativePath],
    ) -> Result<(), anyhow::Error> {
        self.collection
            .insert_schema_multi_sources(content_hash, origin, filepaths)
            .await
    }

    pub(super) async fn insert_schema_no_source(
        &mut self,
        content_hash: &FmtHash,
        origin: &Option<Origin>,
    ) -> Result<(), anyhow::Error> {
        self.collection
            .insert_schema_no_source(content_hash, origin)
            .await
    }

    pub(super) async fn insert_schema_source(
        &mut self,
        content_hash: &FmtHash,
        filepath: &RelativePath,
    ) -> Result<(), anyhow::Error> {
        self.collection
            .insert_schema_source(content_hash, filepath)
            .await
    }

    pub(super) async fn insert_collect_cfg<C: serde::Serialize>(
        &mut self,
        cfg: &C,
        origin: &Option<Origin>,
    ) -> Result<i64, anyhow::Error> {
        let cfg_hash = FmtHash::from(cfg);
        self.insert_general_json(&cfg_hash, &serde_json::to_value(cfg)?)
            .await?;

        let origin_hash = if let Some(origin_value) = &origin {
            let origin_hash = FmtHash::from(origin_value);
            self.insert_general_json(&origin_hash, origin_value).await?;
            Some(origin_hash)
        } else {
            None
        };

        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        let cfg_nr = sqlx::query!(
            "
            select count(*) as nr
            from CollectConfigs
            where collect_nr = $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .fetch_one(self.connection_mut())
        .await
        .context("Failed querying nr of collected configs")?
        .nr + 1; // +1, because the newly added will become the new maximum

        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into CollectConfigs (
                collect_nr,
                product_id,
                nr,
                cfg_hash,
                origin_hash
            )
            values (
                $1,
                $2,
                $3,
                $4,
                $5
            )
            ",
            collect_nr,
            product_id,
            cfg_nr,
            cfg_hash,
            origin_hash
        )
        .execute(self.connection_mut())
        .await
        .context("Failed inserting a collect config")?;

        Ok(cfg_nr)
    }

    pub(super) async fn insert_cfg_collected_schema(
        &mut self,
        cfg_nr: i64,
        schema_hash: &FmtHash,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or ignore into ConfigCollectedSchemas (
                collect_nr,
                product_id,
                schema_hash,
                cfg_nr
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
            schema_hash,
            cfg_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed inserting a collect config")?;

        Ok(())
    }

    /// Returns the absolute path to the directory the used mantra config file is located in.
    pub(super) fn abs_cfg_file_parent_path(&self) -> PathBuf {
        self.collection.abs_cfg_file_parent_path()
    }
}
