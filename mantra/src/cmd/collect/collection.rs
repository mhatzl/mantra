use std::path::PathBuf;

use anyhow::Context;
use mantra_schema::{FmtHash, Origin, path::RelativePath, time::OffsetDateTime};

use crate::{
    cfg::ResolvedProductConfig,
    cmd::collect::{
        SingleFileCollector, cfg::CollectConfig, product_collection::ProductCollection,
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

    pub(super) async fn collect_product(
        &mut self,
        product_cfg: ResolvedProductConfig,
    ) -> Result<(), anyhow::Error> {
        let product_collection = ProductCollection::new(self, &product_cfg.product).await?;

        let req_collector = SingleFileCollector::new(product_collection);
        let mut product_collection = req_collector
            .collect(product_cfg.requirements)
            .await
            .context("Failed to collect requirements")?;

        let annotation_collector = SingleFileCollector::new(product_collection);
        let mut product_collection = annotation_collector
            .collect(product_cfg.annotations)
            .await
            .context("Failed to collect annotations")?;
        product_collection
            .resolve_element_identifier(product_cfg.lsif)
            .await
            .context("Failed to resolve element identifiers")?;

        super::test_runs::collect(&mut product_collection, product_cfg.test_runs)
            .await
            .context("Failed to collect test runs")?;

        let review_collector = SingleFileCollector::new(product_collection);
        let mut product_collection = review_collector
            .collect(product_cfg.reviews)
            .await
            .context("Failed to collect reviews")?;

        Ok(())
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

        sqlx::query!(
            "
            insert into CollectedFiles (
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
        file_hash: &FmtHash,
        content: Option<&str>,
    ) -> Result<(), anyhow::Error> {
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
            file_hash,
            content
        )
        .execute(self.connection_mut())
        .await?;

        let filepath = filepath.as_str();
        let collect_nr = self.collect_nr();

        if sqlx::query!(
            "
            select file_hash
            from CollectedFiles
            where collect_nr = $1 and filepath = $3 and file_hash != $4
            ",
            collect_nr,
            filepath,
            file_hash
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Failed to get collected files")?
        .and_then(|r| r.file_hash)
        .is_some()
        {
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
                file_hash
            )
            .execute(self.connection_mut())
            .await
            .context("Failed to update conflicting collected files")?;
        } else {
            sqlx::query!(
                "
                insert into CollectedFiles (
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
                file_hash
            )
            .execute(self.connection_mut())
            .await
            .context("Failed to update collected files")?;
        }

        Ok(())
    }

    pub(super) async fn insert_schema_single_source(
        &mut self,
        content_hash: &FmtHash,
        origin: &Option<Origin>,
        filepath: &RelativePath,
    ) -> Result<(), anyhow::Error> {
        self.insert_schema(content_hash, origin).await?;

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
            content_hash,
            filepath
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update collected schema sources")?;

        Ok(())
    }

    pub(super) async fn insert_schema_multi_sources(
        &mut self,
        content_hash: &FmtHash,
        origin: &Option<Origin>,
        filepaths: &[&RelativePath],
    ) -> Result<(), anyhow::Error> {
        self.insert_schema(content_hash, origin).await?;

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
                content_hash,
                filepath
            )
            .execute(self.connection_mut())
            .await
            .context("Failed to update collected schema sources")?;
        }

        Ok(())
    }

    pub(super) async fn insert_schema_no_source(
        &mut self,
        content_hash: &FmtHash,
        origin: &Option<Origin>,
    ) -> Result<(), anyhow::Error> {
        self.insert_schema(content_hash, origin).await
    }

    pub(super) async fn insert_schema_source(
        &mut self,
        content_hash: &FmtHash,
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
            content_hash,
            filepath
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update collected schema sources")?;

        Ok(())
    }

    async fn insert_schema(
        &mut self,
        content_hash: &FmtHash,
        origin: &Option<Origin>,
    ) -> Result<(), anyhow::Error> {
        let origin_hash = if let Some(origin_value) = &origin {
            let origin_hash = FmtHash::from(origin_value);
            self.insert_general_json(&origin_hash, origin_value).await?;
            Some(origin_hash)
        } else {
            None
        };

        // TODO: flag if potential existing origin differs
        // Should not happen, since origin is part of the hash, so indicates an hashing error
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
            content_hash,
            origin_hash
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update collected schemas")?;

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
