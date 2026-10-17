use std::path::PathBuf;

use anyhow::Context;
use mantra_schema::{
    FmtHash, Schema,
    path::{RelativePath, RelativePathBuf},
    product::{Product, ProductId},
};

use crate::{
    cfg::ResolvedProductConfig,
    cmd::collect::{SingleFileCollector, collection::Collection},
    db::MantraConnection,
};

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

    pub(super) async fn collect(
        mut self,
        product_cfg: ResolvedProductConfig,
    ) -> Result<(), anyhow::Error> {
        let collected_req_cfgs = self
            .insert_requirement_collect_cfgs(product_cfg.requirements)
            .await
            .context("Inserting requirement collection configs")?;
        SingleFileCollector::collect(&mut self, collected_req_cfgs)
            .await
            .context("Failed to collect requirements")?;
        self.update_req_dot_parent()
            .await
            .context("Updating requirement dot-hierarchy")?;

        let collected_annotation_cfgs = self
            .insert_annotation_collect_cfgs(product_cfg.annotations)
            .await
            .context("Inserting annotation collection config")?;
        SingleFileCollector::collect(&mut self, collected_annotation_cfgs)
            .await
            .context("Failed to collect annotations")?;
        // TODO
        // product_collection
        //     .resolve_element_identifier(product_cfg.lsif)
        //     .await
        //     .context("Failed to resolve element identifiers")?;

        // Calling after requirements and annotations have been collected,
        // because update depends on latest entries from both requirements and annotations.
        //
        // TODO: Skip if neither requirements or annotations are collected
        self.update_direct_req_traces()
            .await
            .context("Updating direct requirement traces")?;

        let test_run_cfgs = self
            .insert_test_run_collect_cfgs(product_cfg.test_runs)
            .await
            .context("Inserting test run collection configs")?;
        super::test_runs::collect(&mut self, test_run_cfgs)
            .await
            .context("Failed to collect test runs")?;

        let review_cfgs = self
            .insert_review_collect_cfgs(product_cfg.reviews)
            .await
            .context("Inserting review collection config")?;
        SingleFileCollector::collect(&mut self, review_cfgs)
            .await
            .context("Failed to collect reviews")?;

        Ok(())
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
        file_hash: Option<&FmtHash>,
        content: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        self.collection
            .insert_collected_file(filepath, file_hash, content)
            .await?;

        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();
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

        if let Some(hash) = file_hash.as_ref() {
            // Note: different hashes for the same file are already handled in `Collection`.
            // So having multiple entries with different hashes is fine here.
            sqlx::query!(
                "
                insert or ignore into ProductRelatedHashedFiles (
                    collect_nr,
                    product_id,
                    filepath,
                    file_hash
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
                filepath,
                hash
            )
            .execute(self.connection_mut())
            .await
            .with_context(|| {
                format!("Failed inserting product related hashed file: {}", filepath)
            })?;
        }

        Ok(())
    }

    pub(super) async fn insert_schema_single_sources<S: Schema>(
        &mut self,
        schema: &S,
        filepath: &RelativePath,
        cfg_nr: i64,
    ) -> Result<FmtHash, anyhow::Error> {
        let schema_hash = self
            .collection
            .insert_schema_single_source(schema, filepath)
            .await?;

        self.insert_cfg_collected_schema(cfg_nr, &schema_hash)
            .await?;

        Ok(schema_hash)
    }

    pub(super) async fn insert_schema_multi_sources<S: Schema>(
        &mut self,
        schema: &S,
        filepaths: &[RelativePathBuf],
        cfg_nr: i64,
    ) -> Result<FmtHash, anyhow::Error> {
        let schema_hash = self
            .collection
            .insert_schema_multi_sources(schema, filepaths)
            .await?;

        self.insert_cfg_collected_schema(cfg_nr, &schema_hash)
            .await?;

        Ok(schema_hash)
    }

    pub(super) async fn insert_schema_no_source<S: Schema>(
        &mut self,
        schema: &S,
        cfg_nr: i64,
    ) -> Result<FmtHash, anyhow::Error> {
        let schema_hash = self.collection.insert_schema_no_source(schema).await?;

        self.insert_cfg_collected_schema(cfg_nr, &schema_hash)
            .await?;

        Ok(schema_hash)
    }

    pub(super) async fn insert_schema_source(
        &mut self,
        schema_hash: &FmtHash,
        filepath: &RelativePath,
        cfg_nr: i64,
    ) -> Result<(), anyhow::Error> {
        self.collection
            .insert_schema_source(schema_hash, filepath)
            .await?;

        self.insert_cfg_collected_schema(cfg_nr, schema_hash).await
    }

    pub(super) async fn schema_collected(
        &mut self,
        schema_hash: &FmtHash,
    ) -> Result<bool, anyhow::Error> {
        self.collection.schema_collected(schema_hash).await
    }

    pub(super) async fn new_collect_cfg(&mut self) -> Result<i64, anyhow::Error> {
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
            insert into CollectConfigs (
                collect_nr,
                product_id,
                nr
            )
            values (
                $1,
                $2,
                $3
            )
            ",
            collect_nr,
            product_id,
            cfg_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed inserting a collect config")?;

        Ok(cfg_nr)
    }

    async fn insert_cfg_collected_schema(
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
