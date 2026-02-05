use mantra_schema::{
    path::{RelativePath, RelativePathBuf},
    product::{Product, ProductId},
    time::OffsetDateTime,
    FmtHash,
};

use crate::db::{MantraConnection, MantraDb, MantraTransaction};

// pub mod db;
pub mod products;
pub mod requirements;
pub mod reviews;
pub mod testcov;
pub mod traces;

pub async fn collect<'db>(db: &'db MantraDb, cfg: CollectConfig) -> Result<(), anyhow::Error> {
    let mut collection = Collection::new(db, &cfg).await?;
    collection.update_product(cfg.product).await?;

    Ok(())
}

pub struct CollectConfig {
    filepath: RelativePathBuf,
    args: CollectArguments,
    envs: CollectEnvironmentVariables,
    product: Product,
    sections: Vec<CollectSectionConfig>,
}

pub struct CollectArguments {
    /// `true`: tells mantra to replace previously collected content
    /// even if the stored hash is equal to the new one.
    replace_hashed: bool,
}
pub struct CollectEnvironmentVariables {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CollectSectionConfig {
    Requirements(CollectRequirementsConfig),
    Annotations(CollectAnnotationsConfig),
    TestRuns(CollectTestRunsConfig),
    Reviews(CollectReviewsConfig),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CollectRequirementsConfig {}
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CollectAnnotationsConfig {}
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CollectTestRunsConfig {}
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CollectReviewsConfig {}

struct Collection<'db> {
    transaction: MantraTransaction<'db>,
    nr: i64,
    product_id: ProductId,
    run_at_utc: OffsetDateTime,
    replace_hashed: bool,
}

impl<'db> Collection<'db> {
    async fn new(db: &'db MantraDb, cfg: &CollectConfig) -> Result<Self, anyhow::Error> {
        let run_at_utc = OffsetDateTime::now_utc();
        let mut transaction = db.start_transaction().await?;
        let config_value = serde_json::json!({
            "product": cfg.product,
            "sections": cfg.sections,
        });
        let config_hash = FmtHash::from(&config_value);

        insert_general_json(
            &mut transaction,
            &config_hash,
            config_value,
            cfg.args.replace_hashed,
        )
        .await?;

        let config_filepath = cfg.filepath.as_str();
        sqlx::query!(
            "
            insert into Collections (
                run_at_utc,
                config_filepath,
                config_hash,
                arguments_hash,
                env_vars_hash
            )
            values (
                $1,
                $2,
                $3,
                null,
                null
            )
            ",
            run_at_utc,
            config_filepath,
            config_hash
        )
        .execute(&mut *transaction.as_mut())
        .await?;

        let nr = sqlx::query!(r#"select max(nr) as "nr!" from Collections"#)
            .fetch_one(&mut *transaction.as_mut())
            .await?
            .nr;

        Ok(Self {
            transaction,
            nr,
            product_id: cfg.product.id(),
            run_at_utc,
            replace_hashed: cfg.args.replace_hashed,
        })
    }

    fn connection(&mut self) -> &mut MantraConnection {
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
            .execute(&mut *self.connection())
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
            .execute(&mut *self.connection())
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
        .execute(&mut *self.connection())
        .await?;

        let filepath = filepath.as_str();
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
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
        .execute(&mut *self.connection())
        .await?;

        Ok(())
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
