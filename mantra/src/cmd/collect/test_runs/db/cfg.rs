use anyhow::Context;
use mantra_schema::FmtHash;

use crate::cmd::collect::{cfg::CollectTestRunsConfig, products::collection::ProductCollection};

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn insert_test_run_collect_cfgs(
        &mut self,
        cfgs: Vec<CollectTestRunsConfig>,
    ) -> Result<Vec<(i64, CollectTestRunsConfig)>, anyhow::Error> {
        let mut collected_cfgs = Vec::with_capacity(cfgs.capacity());

        for cfg in cfgs {
            let cfg_nr = self.new_collect_cfg().await?;

            self.insert_test_run_cfg(&cfg, cfg_nr)
                .await
                .context("Inserting test run config")?;

            collected_cfgs.push((cfg_nr, cfg));
        }

        Ok(collected_cfgs)
    }

    async fn insert_test_run_cfg(
        &mut self,
        test_run_cfg: &CollectTestRunsConfig,
        cfg_entry: i64,
    ) -> Result<(), anyhow::Error> {
        let source_value = serde_json::to_value(&test_run_cfg.source)?;
        let source_hash = FmtHash::new_serialized(&source_value);

        self.insert_general_json(&source_hash, &source_value)
            .await
            .context("Inserting source config")?;

        let origin_hash = if let Some(origin) = test_run_cfg.origin.as_ref() {
            let hash = FmtHash::new_serialized(origin);
            self.insert_general_json(&hash, origin)
                .await
                .context("Inserting origin")?;

            Some(hash)
        } else {
            None
        };

        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();
        let path = test_run_cfg.path.as_str();

        sqlx::query!(
            "
            insert into TestRunCollectConfigs (
                latest_collection,
                product_id,
                cfg_entry,
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
            on conflict (
                product_id,
                cfg_entry
            ) do update
            set
                latest_collection = excluded.latest_collection,
                path = excluded.path,
                source_hash = excluded.source_hash,
                origin_hash = excluded.origin_hash,
                pattern = excluded.pattern
            ",
            collect_nr,
            product_id,
            cfg_entry,
            path,
            source_hash,
            origin_hash,
            test_run_cfg.pattern
        )
        .execute(self.connection_mut())
        .await
        .context("Inserting test run collect config")?;

        if let Some(properties) = &test_run_cfg.test_run_properties {
            for (key, value) in properties {
                let value_hash = FmtHash::new_serialized(value);
                self.insert_general_json(&value_hash, value)
                    .await
                    .with_context(|| format!("Inserting value for property '{key}'"))?;

                sqlx::query!(
                    "
                    insert into ConfigTestRunProperties (
                        latest_collection,
                        product_id,
                        cfg_entry,
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
                    on conflict (
                        product_id,
                        cfg_entry,
                        property_key
                    ) do update
                    set
                        latest_collection = excluded.latest_collection,
                        value_hash = excluded.value_hash
                    ",
                    collect_nr,
                    product_id,
                    cfg_entry,
                    key,
                    value_hash
                )
                .execute(self.connection_mut())
                .await
                .context("Inserting test run property '{key}'")?;
            }
        }

        if let Some(properties) = &test_run_cfg.test_case_properties {
            for (key, value) in properties {
                let value_hash = FmtHash::new_serialized(value);
                self.insert_general_json(&value_hash, value)
                    .await
                    .with_context(|| format!("Inserting value for property '{key}'"))?;

                sqlx::query!(
                    "
                    insert into ConfigTestCaseProperties (
                        latest_collection,
                        product_id,
                        cfg_entry,
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
                    on conflict (
                        product_id,
                        cfg_entry,
                        property_key
                    ) do update
                    set
                        latest_collection = excluded.latest_collection,
                        value_hash = excluded.value_hash
                    ",
                    collect_nr,
                    product_id,
                    cfg_entry,
                    key,
                    value_hash
                )
                .execute(self.connection_mut())
                .await
                .context("Inserting test case property '{key}'")?;
            }
        }

        Ok(())
    }
}
