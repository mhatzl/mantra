use anyhow::Context;
use mantra_schema::{
    FmtHash, Properties, path::RelativePath, product::ProductId, time::OffsetDateTime,
};
use std::path::PathBuf;

use crate::{
    cmd::collect::{cfg::CollectConfig, collector::SingleFileCollector},
    db::{MantraConnection, MantraDb, MantraTransaction},
};

pub mod annotations;
pub mod cfg;
pub mod collector;
pub mod db;
pub mod lsif;
pub mod products;
pub mod requirements;
pub mod reviews;
pub mod test_runs;
pub mod walker;

#[cfg(test)]
mod test_setup;

pub async fn collect(db: &MantraDb, cfg: CollectConfig) -> Result<(), anyhow::Error> {
    let mut collection = collect_data(db, cfg).await?;

    collection
        .aggregate_requirements_data()
        .await
        .context("Failed to aggregate requirements data")?;
    collection
        .aggregate_annotations_data()
        .await
        .context("Failed to aggregate annotations data")?;
    collection
        .aggregate_test_run_data()
        .await
        .context("Failed to aggregated test runs data")?;

    collection
        .aggregate_verification_data()
        .await
        .context("Failed to aggregate verification states")?;

    collection
        .commit()
        .await
        .context("Failed to commit the collected data")?;

    Ok(())
}

async fn collect_data<'db>(
    db: &'db MantraDb,
    cfg: CollectConfig,
) -> Result<Collection<'db>, anyhow::Error> {
    let mut collection = Collection::new(db, &cfg)
        .await
        .context("Failed to create the data collector")?;
    collection
        .update_product(cfg.product)
        .await
        .context("Failed to update product data")?;

    let req_collector = SingleFileCollector::new(collection);
    let mut collection = req_collector
        .collect(cfg.requirements)
        .await
        .context("Failed to collect requirements")?;
    // Note: dot-hierarchy updated explicitely after collecting all requirements,
    // because not all *dot-parts* may have been added as requirements.
    // e.g. top.missing.lead-id => skipping "missing" if not available as requirement
    collection
        .update_req_dot_hierarchy()
        .await
        .context("Failed to update the requirements hierarchy")?;
    collection
        .delete_outdated_reqs()
        .await
        .context("Failed to delete outdated requirement data")?;

    let annotation_collector = SingleFileCollector::new(collection);
    let mut collection = annotation_collector
        .collect(cfg.annotations)
        .await
        .context("Failed to collect annotations")?;
    collection
        .resolve_element_identifier(cfg.lsif)
        .await
        .context("Failed to resolve element identifiers")?;
    collection
        .delete_outdated_annotations()
        .await
        .context("Failed to delete outdated annotation data")?;

    test_runs::collect(&mut collection, cfg.test_runs)
        .await
        .context("Failed to collect test runs")?;
    collection
        .delete_outdated_test_runs()
        .await
        .context("Failed to delete outdated test run data")?;

    let review_collector = SingleFileCollector::new(collection);
    let mut collection = review_collector
        .collect(cfg.reviews)
        .await
        .context("Failed to collect reviews")?;
    collection
        .delete_outdated_reviews()
        .await
        .context("Failed to delete outdated review data")?;

    // product cleanup after all other data was collected,
    // because product data may get updated from any source.
    collection
        .delete_outdated_product_info()
        .await
        .context("Failed to delete outdated product data")?;

    Ok(collection)
}

struct Collection<'db> {
    transaction: MantraTransaction<'db>,
    cfg_filepath: PathBuf,
    abs_cfg_file_parent_path: PathBuf,
    nr: i64,
    product_id: ProductId,
    collected_at_utc: OffsetDateTime,
    replace_hashed: bool,
}

impl<'db> Collection<'db> {
    async fn new(db: &'db MantraDb, cfg: &CollectConfig) -> Result<Self, anyhow::Error> {
        let collected_at_utc = OffsetDateTime::now_utc();
        let mut transaction = db.start_transaction().await?;
        let config_value = serde_json::json!({
            "product": cfg.product,
            "req-cfg": cfg.requirements,
            "annotation-cfg": cfg.annotations,
            "test-run-cfg": cfg.test_runs,
            "review-cfg": cfg.reviews
        });
        let config_hash = FmtHash::from(&config_value);

        insert_general_json(
            &mut transaction,
            &config_hash,
            config_value,
            cfg.args.replace_hashed,
        )
        .await
        .context("Failed to insert the product configuration used to collect data")?;

        // TODO: add args and env data to table

        sqlx::query!(
            "
            insert into Collections (
                collected_at_utc,
                config_hash,
                arguments_hash,
                env_vars_hash
            )
            values (
                $1,
                $2,
                null,
                null
            )
            ",
            collected_at_utc,
            config_hash
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
            cfg_filepath: cfg.cfg_filepath.clone(),
            abs_cfg_file_parent_path: crate::io::abs_parent_path(&cfg.cfg_filepath)?,
            nr,
            product_id: cfg.product.id.clone(),
            collected_at_utc,
            replace_hashed: cfg.args.replace_hashed,
        })
    }

    fn connection_mut(&mut self) -> &mut MantraConnection {
        self.transaction.as_mut()
    }

    fn collect_nr(&self) -> i64 {
        self.nr
    }

    fn product_id(&self) -> ProductId {
        self.product_id.clone()
    }

    async fn commit(self) -> Result<(), anyhow::Error> {
        Ok(self.transaction.commit().await?)
    }

    async fn insert_general_json(
        &mut self,
        hash: &FmtHash,
        content: serde_json::Value,
    ) -> Result<(), anyhow::Error> {
        insert_general_json(&mut self.transaction, hash, content, self.replace_hashed).await
    }

    async fn insert_general_text(
        &mut self,
        hash: &FmtHash,
        content: String,
        media_type: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        if self.replace_hashed {
            sqlx::query!(
                "
                insert or replace into GeneralTexts (
                    hash,
                    content,
                    media_type
                )
                values (
                    $1,
                    $2,
                    $3
                )
                ",
                hash,
                content,
                media_type
            )
            .execute(self.connection_mut())
            .await?;
        } else {
            sqlx::query!(
                "
                insert or ignore into GeneralTexts (
                    hash,
                    content,
                    media_type
                )
                values (
                    $1,
                    $2,
                    $3
                )
                ",
                hash,
                content,
                media_type
            )
            .execute(self.connection_mut())
            .await?;
        }

        Ok(())
    }

    async fn insert_file_content(
        &mut self,
        filepath: &RelativePath,
        file_hash: &FmtHash,
        content: &str,
    ) -> Result<(), anyhow::Error> {
        let media_type = mime_guess::from_ext(filepath.extension().unwrap_or_default()).first_raw();

        if self.replace_hashed {
            sqlx::query!(
                "
                insert or replace into GeneralTexts (
                    hash,
                    content,
                    media_type
                )
                values (
                    $1,
                    $2,
                    $3
                )
                ",
                file_hash,
                content,
                media_type
            )
            .execute(self.connection_mut())
            .await?;
        } else {
            sqlx::query!(
                "
                insert or ignore into GeneralTexts (
                    hash,
                    content,
                    media_type
                )
                values (
                    $1,
                    $2,
                    $3
                )
                ",
                file_hash,
                content,
                media_type
            )
            .execute(self.connection_mut())
            .await?;
        }

        Ok(())
    }

    async fn insert_file_hash(
        &mut self,
        filepath: &RelativePath,
        file_hash: &FmtHash,
    ) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            insert or ignore into FileHashes (
                hash
            )
            values (
                $1
            )
            ",
            file_hash
        )
        .execute(self.connection_mut())
        .await?;

        let filepath = filepath.as_str();
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        if sqlx::query!(
            "
            select filepath
            from ProductRelatedFiles
            where last_collect_nr = $1 and product_id = $2
            and filepath = $3 and file_hash != $4
            ",
            collect_nr,
            product_id,
            filepath,
            file_hash
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Failed to get collected product-related files")?
        .is_some()
        {
            anyhow::bail!(
                "Duplicate entry in same collection for filepath '{}' with different file hash.",
                filepath
            );
        } else {
            sqlx::query!(
                "
                insert into ProductRelatedFiles (
                    last_collect_nr,
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
                on conflict (product_id, filepath)
                do update set
                    last_collect_nr = excluded.last_collect_nr,
                    file_hash = excluded.file_hash
                ",
                collect_nr,
                product_id,
                filepath,
                file_hash
            )
            .execute(self.connection_mut())
            .await
            .context("Failed to update product-related files")?;
        }

        Ok(())
    }

    /// Returns the absolute path to the directory the used mantra config file is located in.
    fn abs_cfg_file_parent_path(&self) -> PathBuf {
        self.abs_cfg_file_parent_path.clone()
    }
}

async fn insert_general_json<'db>(
    transaction: &mut MantraTransaction<'db>,
    hash: &FmtHash,
    content: serde_json::Value,
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

fn merge_local_and_base_properties(
    local_props: Option<Properties>,
    base_props: &Option<Properties>,
) -> Option<Properties> {
    if local_props.is_none() && base_props.is_none() {
        return None;
    }

    let mut props = local_props.unwrap_or_default();

    if let Some(base_props) = base_props {
        for base_prop in base_props {
            if !props.contains_key(base_prop.0) {
                props.insert(base_prop.0.clone(), base_prop.1.clone());
            }
        }
    }

    Some(props)
}
