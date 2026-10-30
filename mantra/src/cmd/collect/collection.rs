use std::{collections::HashSet, path::PathBuf};

use anyhow::{Context, bail};
use mantra_schema::{
    FmtHash, Schema,
    media_type::MediaType,
    path::{RelativePath, RelativePathBuf},
    time::OffsetDateTime,
};

use crate::{
    cfg::ResolvedProductConfig,
    cmd::collect::{
        cfg::{CollectArguments, CollectConfig, CollectMode},
        products::collection::ProductCollection,
    },
    db::{MantraConnection, MantraDb, MantraTransaction},
};

pub(crate) struct Collection<'db> {
    transaction: MantraTransaction<'db>,
    cfg_filepath: PathBuf,
    abs_cfg_file_parent_path: PathBuf,
    id: i64,
    collected_at_utc: OffsetDateTime,
    replace_hashed: bool,
}

impl<'db> Collection<'db> {
    pub(super) async fn new(db: &'db MantraDb, cfg: &CollectConfig) -> Result<Self, anyhow::Error> {
        let collected_at_utc = OffsetDateTime::now_utc();
        let mut transaction = db.start_transaction().await?;

        // TODO: add args and env data to table

        sqlx::query!(
            "
            insert into Collections (
                collected_at_utc,
                arguments_hash,
                env_vars_hash
            )
            values (
                $1,
                null,
                null
            )
            ",
            collected_at_utc
        )
        .execute(&mut *transaction.as_mut())
        .await
        .context("Failed to create a new collection")?;

        let id = sqlx::query!(r#"select max(id) as "id!" from Collections"#)
            .fetch_one(&mut *transaction.as_mut())
            .await
            .context("Failed to get the latest collection")?
            .id;

        let mut collection = Self {
            transaction,
            cfg_filepath: cfg.cfg_filepath().to_path_buf(),
            abs_cfg_file_parent_path: crate::io::abs_parent_path(cfg.cfg_filepath())?,
            id,
            collected_at_utc,
            replace_hashed: cfg.args().replace_hashed,
        };

        collection
            .reset_changes()
            .await
            .context("Resetting history")?;

        Ok(collection)
    }

    pub(super) async fn collect(&mut self, cfg: CollectConfig) -> Result<(), anyhow::Error> {
        self.collect_products(&cfg.args, cfg.product_cfgs).await?;

        self.aggregate().await
    }

    pub(super) async fn aggregate(&mut self) -> Result<(), anyhow::Error> {
        self.aggregate_requirements_data()
            .await
            .context("Failed to aggregate requirements data")?;
        self.aggregate_annotations_data()
            .await
            .context("Failed to aggregate annotations data")?;
        self.aggregate_test_run_data()
            .await
            .context("Failed to aggregated test runs data")?;

        self.aggregate_mixed()
            .await
            .context("Aggregating interleaved data")?;

        Ok(())
    }

    pub(super) async fn collect_products(
        &mut self,
        args: &CollectArguments,
        product_cfgs: Vec<ResolvedProductConfig>,
    ) -> Result<(), anyhow::Error> {
        let mut product_map = HashSet::new();

        for mut product_cfg in product_cfgs {
            if !product_map.insert(product_cfg.product.id.clone()) {
                log::warn!(
                    "Product '{}' has more than one product entry that maps to it!",
                    &product_cfg.product.id
                );
            }

            let product_id = product_cfg.product.id.clone();
            let collect_data = if let Some(specific_id) = &args.product_id {
                if specific_id == &product_id {
                    if let Some(base) = &args.product_base {
                        product_cfg.product.base = Some(base.clone());
                    }
                    if let Some(version) = &args.product_version {
                        product_cfg.product.version = Some(version.clone());
                    }

                    true
                } else {
                    false
                }
            } else {
                true
            };

            if collect_data {
                self.collect_product(product_cfg, &args.mode)
                    .await
                    .with_context(|| format!("Failed to collect product '{}'", product_id))?
            }
        }

        if &args.mode == &CollectMode::RemoveMissing {
            self.remove_outdated_products()
                .await
                .context("Remove outdated products")?;
        }

        Ok(())
    }

    async fn collect_product(
        &mut self,
        product_cfg: ResolvedProductConfig,
        collect_mode: &CollectMode,
    ) -> Result<(), anyhow::Error> {
        let product_collection = ProductCollection::new(self, &product_cfg.product)
            .await
            .context("Creating product collection")?;

        product_collection.collect(product_cfg, collect_mode).await
    }

    pub(super) fn connection_mut(&mut self) -> &mut MantraConnection {
        self.transaction.as_mut()
    }

    pub(super) fn collect_nr(&self) -> i64 {
        self.id
    }

    pub(super) async fn commit(self) -> Result<(), anyhow::Error> {
        Ok(self.transaction.commit().await?)
    }

    pub(super) async fn insert_general_json(
        &mut self,
        hash: &FmtHash,
        content: &serde_json::Value,
    ) -> Result<(), anyhow::Error> {
        insert_general_json(&mut self.transaction, hash, content, self.replace_hashed).await
    }

    pub(super) async fn insert_general_text(
        &mut self,
        hash: &FmtHash,
        content: &str,
    ) -> Result<(), anyhow::Error> {
        // Content hash computed internally, so should always be correct
        //debug_assert_eq!(hash, &FmtHash::new(content));
        let expected = FmtHash::new(content);
        if hash != &expected {
            bail!("Unexpected hash '{}'. Expected: {}", hash, expected);
        }

        if self.replace_hashed {
            sqlx::query!(
                "
                insert into GeneralTexts (
                    hash,
                    content
                )
                values (
                    $1,
                    $2
                ) on conflict (hash) do update
                set content = excluded.content
                ",
                hash,
                content
            )
            .execute(self.connection_mut())
            .await?;
        } else {
            sqlx::query!(
                "
                insert or ignore into GeneralTexts (
                    hash,
                    content
                )
                values (
                    $1,
                    $2
                )
                ",
                hash,
                content
            )
            .execute(self.connection_mut())
            .await?;
        }

        Ok(())
    }

    pub(super) async fn collect_unhashed_file(
        &mut self,
        filepath: &RelativePath,
    ) -> Result<(), anyhow::Error> {
        self.insert_collected_file(filepath, None, None).await
    }

    pub(super) async fn insert_collected_file(
        &mut self,
        filepath: &RelativePath,
        file_hash: Option<&FmtHash>,
        content: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let media_type = MediaType::new(filepath);
        let filepath = filepath.as_str();
        let collect_nr = self.collect_nr();

        if let Some(hash) = file_hash
            && let Some(some_content) = content
        {
            let expected_hash = FmtHash::new(some_content);

            if &expected_hash != hash {
                bail!(
                    "Wrong content hash for filepath '{}'. Expected: {}",
                    filepath,
                    expected_hash
                );
            }
        }

        if let Some(hash) = file_hash.as_ref() {
            if self.replace_hashed
                && let Some(some_content) = content
            {
                sqlx::query!(
                    "
                    insert into FileHashes (
                        hash,
                        content
                    )
                    values (
                        $1,
                        $2
                    )
                    on conflict (hash) do update
                    set content = excluded.content
                    ",
                    hash,
                    some_content
                )
                .execute(self.connection_mut())
                .await?;
            } else {
                // either
                sqlx::query!(
                    "
                    insert into FileHashes (
                        hash,
                        content
                    )
                    values (
                        $1,
                        $2
                    )
                    on conflict (hash) do update
                    -- we take the one that is not null
                    set content = coalesce(FileHashes.content, excluded.content)
                    ",
                    hash,
                    content
                )
                .execute(self.connection_mut())
                .await?;
            }

            if let Some(collected_hash) = sqlx::query!(
                "
                select file_hash
                from CollectedFiles
                where latest_collection = $1 and filepath = $2
                and file_hash is not null and file_hash != $3
                ",
                collect_nr,
                filepath,
                hash
            )
            .fetch_optional(self.connection_mut())
            .await
            .context("Failed to get collected files")?
            .and_then(|r| r.file_hash)
            {
                // TODO: Convert to error once stacked errors are used
                log::warn!(
                    "Different file hashes in same collection for filepath '{}'.\n\tPrevious: {}\n\tNew: {}",
                    filepath,
                    collected_hash,
                    hash
                );

                sqlx::query!(
                    "
                    insert into ConflictingCollectedFiles (
                        latest_collection,
                        filepath,
                        file_hash
                    )
                    values (
                        $1,
                        $2,
                        $3
                    )
                    on conflict (
                        filepath,
                        file_hash
                    ) do update
                    set
                        latest_collection = excluded.latest_collection
                    ",
                    collect_nr,
                    filepath,
                    hash
                )
                .execute(self.connection_mut())
                .await
                .context("Failed to update conflicting collected files")?;
            }
        }

        sqlx::query!(
            "
            insert into CollectedFiles (
                latest_collection,
                filepath,
                file_hash,
                media_type
            )
            values (
                $1,
                $2,
                $3,
                $4
            )
            on conflict (filepath) do update
            set
                latest_collection = excluded.latest_collection,
                -- use the non-null value in case file collected in same collection, otherwise overwrite
                file_hash = case
                    when CollectedFiles.latest_collection = excluded.latest_collection
                        then coalesce(CollectedFiles.file_hash, excluded.file_hash)
                    else excluded.file_hash
                end,
                -- always use the latest media-type representation we created above
                -- in case media types based on file extensions changed
                media_type = excluded.file_hash
            ",
            collect_nr,
            filepath,
            file_hash,
            media_type
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to insert collected file")?;

        Ok(())
    }

    pub(super) async fn insert_schema_single_source<S: Schema>(
        &mut self,
        schema: &S,
        filepath: &RelativePath,
    ) -> Result<FmtHash, anyhow::Error> {
        let schema_hash = self.insert_schema(schema).await?;

        self.insert_schema_source(&schema_hash, filepath).await?;

        Ok(schema_hash)
    }

    pub(super) async fn insert_schema_multi_sources<S: Schema>(
        &mut self,
        schema: &S,
        filepaths: &[RelativePathBuf],
    ) -> Result<FmtHash, anyhow::Error> {
        let schema_hash = self.insert_schema(schema).await?;

        for filepath in filepaths {
            self.insert_schema_source(&schema_hash, filepath).await?;
        }

        Ok(schema_hash)
    }

    pub(super) async fn insert_schema_no_source<S: Schema>(
        &mut self,
        schema: &S,
    ) -> Result<FmtHash, anyhow::Error> {
        self.insert_schema(schema).await
    }

    pub(super) async fn insert_schema_source(
        &mut self,
        schema_hash: &FmtHash,
        filepath: &RelativePath,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let filepath = filepath.as_str();

        sqlx::query!(
            "
            insert into SchemaSources (
                latest_collection,
                schema_hash,
                filepath
            )
            values (
                $1,
                $2,
                $3
            )
            on conflict (
                schema_hash,
                filepath
            ) do update
            set
                latest_collection = excluded.latest_collection
            ",
            collect_nr,
            schema_hash,
            filepath
        )
        .execute(self.connection_mut())
        .await
        .with_context(|| format!("Inserting schema source file: {}", filepath))?;

        Ok(())
    }

    pub(super) async fn schema_collected(
        &mut self,
        schema_hash: &FmtHash,
    ) -> Result<bool, anyhow::Error> {
        Ok(sqlx::query!(
            "
            select *
            from Schemas
            where content_hash = $1
            ",
            schema_hash
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Failed to get collected schemas")?
        .is_some())
    }

    async fn insert_schema<S: Schema>(&mut self, schema: &S) -> Result<FmtHash, anyhow::Error> {
        let schema_hash = FmtHash::new_serialized(schema);

        let origin_hash = if let Some(origin_value) = schema.origin() {
            let origin_hash = FmtHash::new_serialized(origin_value);
            self.insert_general_json(&origin_hash, origin_value).await?;
            Some(origin_hash)
        } else {
            None
        };

        if let Some(record) = sqlx::query!(
            "
            select origin_hash
            from Schemas
            where content_hash = $1
            ",
            schema_hash
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying collected schemas")?
        {
            if record.origin_hash.map(FmtHash::with_inner) != origin_hash {
                bail!(
                    "Schema previously collected with different origin, which indicates hashing mistake, because the origin is part of the schema"
                );
            } else {
                // Schema already inserted
                return Ok(schema_hash);
            }
        }

        sqlx::query!(
            "
            insert into Schemas (
                content_hash,
                origin_hash
            )
            values (
                $1,
                $2
            )
            ",
            schema_hash,
            origin_hash
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update collected schemas")?;

        Ok(schema_hash)
    }

    /// Deletes changes made by a previous collection.
    /// The "history" tables can only track the changes made by the new collection.
    async fn reset_changes(&mut self) -> Result<(), anyhow::Error> {
        self.reset_requirement_changes()
            .await
            .context("Resetting requirement history")?;

        Ok(())
    }

    /// Returns the absolute path to the directory the used mantra config file is located in.
    pub(super) fn abs_cfg_file_parent_path(&self) -> PathBuf {
        self.abs_cfg_file_parent_path.clone()
    }
}

async fn insert_general_json<'db>(
    transaction: &mut MantraTransaction<'db>,
    hash: &FmtHash,
    content: &serde_json::Value,
    replace_hashed: bool,
) -> Result<(), anyhow::Error> {
    // Content hash computed internally, so should always be correct
    //debug_assert_eq!(hash, &FmtHash::from(content));
    let expected = FmtHash::new_serialized(content);
    if hash != &expected {
        bail!("Unexpected hash '{}'. Expected: {}", hash, expected);
    }

    if replace_hashed {
        sqlx::query!(
            "
            insert into GeneralJson (
                hash,
                content
            )
            values (
                $1,
                $2
            )
            on conflict (hash) do update
            set content = excluded.content
            ",
            hash,
            content
        )
        .execute(&mut *transaction.as_mut())
        .await?;
    } else {
        sqlx::query!(
            "
            insert or ignore into GeneralJson (
                hash,
                content
            )
            values (
                $1,
                $2
            )
            ",
            hash,
            content
        )
        .execute(&mut *transaction.as_mut())
        .await?;
    }

    Ok(())
}
