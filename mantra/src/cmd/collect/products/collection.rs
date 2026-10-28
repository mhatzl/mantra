use std::path::PathBuf;

use anyhow::Context;
use mantra_schema::{
    FmtHash, Schema,
    path::{RelativePath, RelativePathBuf},
    product::{Product, ProductId},
};

use crate::{
    cfg::ResolvedProductConfig,
    cmd::collect::{SingleFileCollector, cfg::CollectMode, collection::Collection},
    db::MantraConnection,
};

pub(crate) struct ProductCollection<'db, 'c> {
    collection: &'c mut Collection<'db>,
    product_id: ProductId,
}

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn new(
        collection: &'c mut Collection<'db>,
        product: &Product,
    ) -> Result<Self, anyhow::Error> {
        let mut pc = Self {
            collection,
            product_id: product.id.clone(),
        };

        pc.collect_product_metadata(product)
            .await
            .context("Failed collecting product metadata")?;

        Ok(pc)
    }

    pub(crate) async fn collect(
        mut self,
        product_cfg: ResolvedProductConfig,
        collect_mode: &CollectMode,
    ) -> Result<(), anyhow::Error> {
        // ----- Requirements ----
        let collected_req_cfgs = self
            .insert_requirement_collect_cfgs(product_cfg.requirements)
            .await
            .context("Inserting requirement collection configs")?;
        SingleFileCollector::collect(&mut self, collected_req_cfgs)
            .await
            .context("Failed to collect requirements")?;

        // Note: Done before updating dot-parents, so only existing requirements are checked for potential parents
        if collect_mode == &CollectMode::RemoveMissing {
            self.remove_outdated_requirement_parts()
                .await
                .context("Removing outdated requirement parts")?;
        }

        self.update_req_dot_parent()
            .await
            .context("Updating requirement dot-hierarchy")?;

        // ----- Annotations ----

        let collected_annotation_cfgs = self
            .insert_annotation_collect_cfgs(product_cfg.annotations)
            .await
            .context("Inserting annotation collection config")?;
        SingleFileCollector::collect(&mut self, collected_annotation_cfgs)
            .await
            .context("Failed to collect annotations")?;

        if collect_mode == &CollectMode::RemoveMissing {
            self.remove_outdated_annotation_parts()
                .await
                .context("Removing outdated annotation parts")?;
        }
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

        // ----- Test Runs ----

        let test_run_cfgs = self
            .insert_test_run_collect_cfgs(product_cfg.test_runs)
            .await
            .context("Inserting test run collection configs")?;
        crate::cmd::collect::test_runs::collect(&mut self, test_run_cfgs)
            .await
            .context("Failed to collect test runs")?;

        if collect_mode == &CollectMode::RemoveMissing {
            self.remove_outdated_test_run_parts()
                .await
                .context("Removing outdated test run parts")?;
        }

        // ----- Reviews ----

        let review_cfgs = self
            .insert_review_collect_cfgs(product_cfg.reviews)
            .await
            .context("Inserting review collection config")?;
        SingleFileCollector::collect(&mut self, review_cfgs)
            .await
            .context("Failed to collect reviews")?;

        if collect_mode == &CollectMode::RemoveMissing {
            self.remove_outdated_review_parts()
                .await
                .context("Removing outdated review parts")?;
        }

        Ok(())
    }

    pub(crate) fn connection_mut(&mut self) -> &mut MantraConnection {
        self.collection.connection_mut()
    }

    pub(crate) fn collect_nr(&self) -> i64 {
        self.collection.collect_nr()
    }

    pub(crate) fn product_id(&self) -> &ProductId {
        &self.product_id
    }

    pub(crate) async fn insert_general_json(
        &mut self,
        hash: &FmtHash,
        content: &serde_json::Value,
    ) -> Result<(), anyhow::Error> {
        self.collection.insert_general_json(hash, content).await
    }

    pub(crate) async fn insert_general_text(
        &mut self,
        hash: &FmtHash,
        content: &str,
    ) -> Result<(), anyhow::Error> {
        self.collection.insert_general_text(hash, content).await
    }

    pub(crate) async fn collect_unhashed_file(
        &mut self,
        filepath: &RelativePath,
    ) -> Result<(), anyhow::Error> {
        self.collection.collect_unhashed_file(filepath).await
    }

    pub(crate) async fn insert_collected_file(
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
            insert into ProductRelatedFiles (
                latest_collection,
                product_id,
                filepath
            )
            values (
                $1,
                $2,
                $3
            )
            on conflict (
                product_id,
                filepath
            ) do update
            set
                latest_collection = excluded.latest_collection
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
            // So always overwriting file_hash here is fine, because it is not 'null'.
            sqlx::query!(
                "
                insert into ProductRelatedHashedFiles (
                    latest_collection,
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
                on conflict (
                    product_id,
                    filepath
                ) do update
                set
                    latest_collection = excluded.latest_collection,
                    file_hash = excluded.file_hash
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

    pub(crate) async fn insert_schema_single_sources<S: Schema>(
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

    pub(crate) async fn insert_schema_multi_sources<S: Schema>(
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

    pub(crate) async fn insert_schema_no_source<S: Schema>(
        &mut self,
        schema: &S,
        cfg_nr: i64,
    ) -> Result<FmtHash, anyhow::Error> {
        let schema_hash = self.collection.insert_schema_no_source(schema).await?;

        self.insert_cfg_collected_schema(cfg_nr, &schema_hash)
            .await?;

        Ok(schema_hash)
    }

    pub(crate) async fn insert_schema_source(
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

    pub(crate) async fn schema_collected(
        &mut self,
        schema_hash: &FmtHash,
    ) -> Result<bool, anyhow::Error> {
        self.collection.schema_collected(schema_hash).await
    }

    pub(crate) async fn new_collect_cfg(&mut self) -> Result<i64, anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        let cfg_entry = sqlx::query!(
            "
            select count(*) as entry_number
            from CollectConfigs
            where latest_collection = $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .fetch_one(self.connection_mut())
        .await
        .context("Failed querying entry_number of collected configs")?
        .entry_number
            + 1; // +1, because the newly added will become the new maximum

        let product_id = self.product_id();

        sqlx::query!(
            "
            insert into CollectConfigs (
                latest_collection,
                product_id,
                entry_number
            )
            values (
                $1,
                $2,
                $3
            )
            on conflict (
                product_id,
                entry_number
            ) do update
            set
                latest_collection = excluded.latest_collection
            ",
            collect_nr,
            product_id,
            cfg_entry
        )
        .execute(self.connection_mut())
        .await
        .context("Failed inserting a collect config")?;

        Ok(cfg_entry)
    }

    async fn insert_cfg_collected_schema(
        &mut self,
        cfg_entry: i64,
        schema_hash: &FmtHash,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert into ConfigCollectedSchemas (
                latest_collection,
                product_id,
                schema_hash,
                cfg_entry
            )
            values (
                $1,
                $2,
                $3,
                $4
            )
            on conflict (
                product_id,
                schema_hash,
                cfg_entry
            ) do update
            set
                latest_collection = excluded.latest_collection
            ",
            collect_nr,
            product_id,
            schema_hash,
            cfg_entry
        )
        .execute(self.connection_mut())
        .await
        .context("Failed inserting a collect config")?;

        Ok(())
    }

    /// Returns the absolute path to the directory the used mantra config file is located in.
    pub(crate) fn abs_cfg_file_parent_path(&self) -> PathBuf {
        self.collection.abs_cfg_file_parent_path()
    }
}
