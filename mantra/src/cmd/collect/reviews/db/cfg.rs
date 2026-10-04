use anyhow::Context;
use mantra_schema::FmtHash;

use crate::cmd::collect::{cfg::CollectReviewsConfig, product_collection::ProductCollection};

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn insert_review_collect_cfgs(
        &mut self,
        cfgs: Vec<CollectReviewsConfig>,
    ) -> Result<Vec<(i64, CollectReviewsConfig)>, anyhow::Error> {
        let mut collected_cfgs = Vec::with_capacity(cfgs.capacity());

        for cfg in cfgs {
            let cfg_nr = self.new_collect_cfg().await?;

            self.insert_review_cfg(&cfg, cfg_nr)
                .await
                .context("Inserting review config")?;

            collected_cfgs.push((cfg_nr, cfg));
        }

        Ok(collected_cfgs)
    }

    async fn insert_review_cfg(
        &mut self,
        review_cfg: &CollectReviewsConfig,
        cfg_nr: i64,
    ) -> Result<(), anyhow::Error> {
        let source_hash = FmtHash::from(&review_cfg.source);
        let source_value = serde_json::to_value(&review_cfg.source)?;
        self.insert_general_json(&source_hash, &source_value)
            .await
            .context("Inserting source config")?;

        let origin_hash = if let Some(origin) = review_cfg.origin.as_ref() {
            let hash = FmtHash::from(origin);
            self.insert_general_json(&hash, origin)
                .await
                .context("Inserting origin")?;

            Some(hash)
        } else {
            None
        };

        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();
        let path = review_cfg.path.as_str();

        sqlx::query!(
            "
            insert into ReviewCollectConfigs (
                collect_nr,
                product_id,
                cfg_nr,
                path,
                source_hash,
                origin_hash,
                pattern
            )
            values (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7
            )
            ",
            collect_nr,
            product_id,
            cfg_nr,
            path,
            source_hash,
            origin_hash,
            review_cfg.pattern
        )
        .execute(self.connection_mut())
        .await
        .context("Inserting review collect config")?;

        if let Some(properties) = &review_cfg.properties {
            for (key, value) in properties {
                let value_hash = FmtHash::from(value);
                self.insert_general_json(&value_hash, value)
                    .await
                    .with_context(|| format!("Inserting value for property '{key}'"))?;

                sqlx::query!(
                    "
                    insert into ConfigReviewProperties (
                        collect_nr,
                        product_id,
                        cfg_nr,
                        property_key,
                        value_hash
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
                    key,
                    value_hash
                )
                .execute(self.connection_mut())
                .await
                .context("Inserting review property '{key}'")?;
            }
        }

        Ok(())
    }
}
