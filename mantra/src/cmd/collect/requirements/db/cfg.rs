use anyhow::Context;
use mantra_schema::FmtHash;

use crate::cmd::collect::{cfg::CollectRequirementsConfig, product_collection::ProductCollection};

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn insert_requirement_collect_cfgs(
        &mut self,
        cfgs: Vec<CollectRequirementsConfig>,
    ) -> Result<Vec<(i64, CollectRequirementsConfig)>, anyhow::Error> {
        let mut collected_cfgs = Vec::with_capacity(cfgs.capacity());

        for cfg in cfgs {
            let cfg_nr = self.new_collect_cfg().await?;

            self.insert_requirement_cfg(&cfg, cfg_nr)
                .await
                .context("Inserting requirement config")?;

            collected_cfgs.push((cfg_nr, cfg));
        }

        Ok(collected_cfgs)
    }

    async fn insert_requirement_cfg(
        &mut self,
        req_cfg: &CollectRequirementsConfig,
        cfg_nr: i64,
    ) -> Result<(), anyhow::Error> {
        let source_hash = FmtHash::from(&req_cfg.source);
        let source_value = serde_json::to_value(&req_cfg.source)?;
        self.insert_general_json(&source_hash, &source_value)
            .await
            .context("Inserting source config")?;

        let origin_hash = if let Some(origin) = req_cfg.origin.as_ref() {
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
        let path = req_cfg.path.as_str();

        sqlx::query!(
            "
            insert into RequirementCollectConfigs (
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
            req_cfg.pattern
        )
        .execute(self.connection_mut())
        .await
        .context("Inserting requirement collect config")?;

        if let Some(properties) = &req_cfg.properties {
            for (key, value) in properties {
                let value_hash = FmtHash::from(value);
                self.insert_general_json(&value_hash, value)
                    .await
                    .with_context(|| format!("Inserting value for property '{key}'"))?;

                sqlx::query!(
                    "
                    insert into ConfigRequirementProperties (
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
                .context("Inserting requirement property '{key}'")?;
            }
        }

        Ok(())
    }
}
