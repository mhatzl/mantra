use std::{collections::HashSet, path::PathBuf};

use anyhow::Context;
use mantra_schema::{
    FmtHash, Schema,
    path::{RelativePath, RelativePathBuf},
    time::OffsetDateTime,
};

use crate::{
    cfg::ResolvedProductConfig,
    cmd::collect::{
        cfg::{CollectArguments, CollectConfig},
        product_collection::ProductCollection,
    },
    db::{MantraConnection, MantraDb, MantraTransaction},
};

pub(crate) struct Collection<'db> {
    transaction: MantraTransaction<'db>,
    cfg_filepath: PathBuf,
    abs_cfg_file_parent_path: PathBuf,
    nr: i64,
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

        let nr = sqlx::query!(r#"select max(nr) as "nr!" from Collections"#)
            .fetch_one(&mut *transaction.as_mut())
            .await
            .context("Failed to get the latest collection")?
            .nr;

        Ok(Self {
            transaction,
            cfg_filepath: cfg.cfg_filepath().to_path_buf(),
            abs_cfg_file_parent_path: crate::io::abs_parent_path(cfg.cfg_filepath())?,
            nr,
            collected_at_utc,
            replace_hashed: cfg.args().replace_hashed,
        })
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
                self.collect_product(product_cfg)
                    .await
                    .with_context(|| format!("Failed to collect product '{}'", product_id))?
            }
        }

        Ok(())
    }

    async fn collect_product(
        &mut self,
        product_cfg: ResolvedProductConfig,
    ) -> Result<(), anyhow::Error> {
        let product_collection = ProductCollection::new(self, &product_cfg.product)
            .await
            .context("Creating product collection")?;

        product_collection.collect(product_cfg).await
    }

    pub(super) fn connection_mut(&mut self) -> &mut MantraConnection {
        self.transaction.as_mut()
    }

    pub(super) fn collect_nr(&self) -> i64 {
        self.nr
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
        if self.replace_hashed {
            sqlx::query!(
                "
                insert or replace into GeneralTexts (
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
        } else if sqlx::query!(
            "
            select content
            from GeneralTexts
            where hash = $1 and content != $2
            ",
            hash,
            content
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Failed to get collected content for general text hashes")?
        .is_some()
        {
            self.insert_conflicting_hashed_content(hash, content)
                .await
                .context("Inserting conflicting hashed general text")?;
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
        let filepath = filepath.as_str();
        let collect_nr = self.collect_nr();

        // Ignoring insert to not overwrite potentially existhing hashes
        sqlx::query!(
            "
            insert or ignore into CollectedFiles (
                collect_nr,
                filepath,
                file_hash
            )
            values (
                $1,
                $2,
                null
            )
            ",
            collect_nr,
            filepath
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update collected files")?;

        Ok(())
    }

    pub(super) async fn insert_collected_file(
        &mut self,
        filepath: &RelativePath,
        file_hash: Option<&FmtHash>,
        content: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let filepath = filepath.as_str();
        let collect_nr = self.collect_nr();

        if let Some(hash) = file_hash.as_ref() {
            if !self.replace_hashed
                && let Some(some_content) = content
                && sqlx::query!(
                    "
                select content
                from FileHashes
                where hash = $1 and content is not null
                and content != $2
                ",
                    hash,
                    some_content
                )
                .fetch_optional(self.connection_mut())
                .await
                .context("Failed to get collected content for file hashes")?
                .and_then(|r| r.content)
                .is_some()
            {
                self.insert_conflicting_hashed_content(hash, some_content)
                    .await
                    .context("Inserting conflicting hashed file content")?;
            } else {
                // Check to not overwrite potential content for an existing entry
                if content.is_some() {
                    sqlx::query!(
                        "
                        insert or ignore into FileHashes (
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
                } else {
                    sqlx::query!(
                        "
                        insert or replace into FileHashes (
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
            }

            if sqlx::query!(
                "
                select file_hash
                from CollectedFiles
                where collect_nr = $1 and filepath = $3
                and file_hash is not null and file_hash != $4
                ",
                collect_nr,
                filepath,
                hash
            )
            .fetch_optional(self.connection_mut())
            .await
            .context("Failed to get collected files")?
            .and_then(|r| r.file_hash)
            .is_some()
            {
                // TODO: Convert to error once stacked errors are used
                log::warn!(
                    "Duplicate entry in same collection for filepath '{}' with different file hash.",
                    filepath
                );

                sqlx::query!(
                    "
                    insert into ConflictingCollectedFiles (
                        collect_nr,
                        filepath,
                        file_hash
                    )
                    values (
                        $1,
                        $2,
                        $3
                    )
                    ",
                    collect_nr,
                    filepath,
                    hash
                )
                .execute(self.connection_mut())
                .await
                .context("Failed to update conflicting collected files")?;
            } else {
                // Either no existing entry, or stored file_hash is null, so replace
                sqlx::query!(
                    "
                    insert or replace into CollectedFiles (
                        collect_nr,
                        filepath,
                        file_hash
                    )
                    values (
                        $1,
                        $2,
                        $3
                    )
                    ",
                    collect_nr,
                    filepath,
                    hash
                )
                .execute(self.connection_mut())
                .await
                .context("Failed to update collected files")?;
            }
        } else {
            // Important to ignore here to not overwrite potentially existing entries with a set file_hash
            sqlx::query!(
                "
                insert or ignore into CollectedFiles (
                    collect_nr,
                    filepath,
                    file_hash
                )
                values (
                    $1,
                    $2,
                    null
                )
                ",
                collect_nr,
                filepath
            )
            .execute(self.connection_mut())
            .await
            .context("Failed to update collected files")?;
        }

        Ok(())
    }

    async fn insert_conflicting_hashed_content(
        &mut self,
        hash: &FmtHash,
        content: &str,
    ) -> Result<(), anyhow::Error> {
        insert_conflicting_hashed_content(&mut self.transaction, hash, content).await
    }

    pub(super) async fn insert_schema_single_source<S: Schema>(
        &mut self,
        schema: &S,
        filepath: &RelativePath,
    ) -> Result<FmtHash, anyhow::Error> {
        let schema_hash = self.insert_schema(schema).await?;

        let collect_nr = self.collect_nr();
        let filepath = filepath.as_str();

        sqlx::query!(
            "
            insert or ignore into SchemaSources (
                collect_nr,
                schema_hash,
                filepath
            )
            values (
                $1,
                $2,
                $3
            )
            ",
            collect_nr,
            schema_hash,
            filepath
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update collected schema sources")?;

        Ok(schema_hash)
    }

    pub(super) async fn insert_schema_multi_sources<S: Schema>(
        &mut self,
        schema: &S,
        filepaths: &[RelativePathBuf],
    ) -> Result<FmtHash, anyhow::Error> {
        let schema_hash = self.insert_schema(schema).await?;

        let collect_nr = self.collect_nr();

        for filepath in filepaths {
            let filepath = filepath.as_str();

            sqlx::query!(
                "
                insert or ignore into SchemaSources (
                    collect_nr,
                    schema_hash,
                    filepath
                )
                values (
                    $1,
                    $2,
                    $3
                )
                ",
                collect_nr,
                schema_hash,
                filepath
            )
            .execute(self.connection_mut())
            .await
            .context("Failed to update collected schema sources")?;
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
            insert or ignore into SchemaSources (
                collect_nr,
                schema_hash,
                filepath
            )
            values (
                $1,
                $2,
                $3
            )
            ",
            collect_nr,
            schema_hash,
            filepath
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update collected schema sources")?;

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
        let schema_hash = FmtHash::from(schema);

        let origin_hash = if let Some(origin_value) = schema.origin() {
            let origin_hash = FmtHash::from(origin_value);
            self.insert_general_json(&origin_hash, origin_value).await?;
            Some(origin_hash)
        } else {
            None
        };

        // TODO: flag if potential existing origin differs
        // Should not happen, since origin is part of the hash, so indicates a hashing error
        sqlx::query!(
            "
            insert or replace into Schemas (
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
    if replace_hashed {
        sqlx::query!(
            "
            insert or replace into GeneralJson (
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
    } else if sqlx::query!(
        "
        select content
        from GeneralJson
        where hash = $1 and content != $2
        ",
        hash,
        content
    )
    .fetch_optional(&mut *transaction.as_mut())
    .await
    .context("Failed to get collected content for general JSON hashes")?
    .is_some()
    {
        insert_conflicting_hashed_content(transaction, hash, content)
            .await
            .context("Inserting conflicting hashed general JSON")?;
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

async fn insert_conflicting_hashed_content<'db>(
    transaction: &mut MantraTransaction<'db>,
    hash: &FmtHash,
    content: impl sqlx::Type<crate::db::SqlDb> + sqlx::Encode<'_, crate::db::SqlDb>,
) -> Result<(), anyhow::Error> {
    // TODO: Convert to error once stacked errors are used
    log::warn!(
        "Duplicate entry in same collection for hash '{}' with different content.",
        hash
    );

    sqlx::query!(
        "
        insert into ConflictingHashes (
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
    .await
    .context("Failed to update conflicting collected hashed content")?;

    Ok(())
}
