use anyhow::Context;
use mantra_schema::{
    FmtHash, Properties, path::RelativePath, product::ProductId, time::OffsetDateTime,
};
use std::{collections::HashSet, path::PathBuf};

use crate::{
    cmd::collect::{cfg::CollectConfig, collection::Collection, collector::SingleFileCollector},
    db::{MantraConnection, MantraDb, MantraTransaction},
};

pub mod annotations;
pub mod cfg;
pub mod collection;
pub mod collector;
pub mod db;
pub mod lsif;
pub mod markup;
pub mod product_collection;
pub mod products;
pub mod requirements;
pub mod reviews;
pub mod test_runs;
pub mod walker;

#[cfg(test)]
mod test_setup;

pub async fn collect(db: &MantraDb, cfg: CollectConfig) -> Result<(), anyhow::Error> {
    let mut product_map = HashSet::new();

    let mut collection = Collection::new(db, &cfg)
        .await
        .context("Failed to start a new collection")?;

    for mut product_cfg in cfg.product_cfgs {
        if !product_map.insert(product_cfg.product.id.clone()) {
            log::warn!(
                "Product '{}' has more than one product entry that maps to it!",
                &product_cfg.product.id
            );
        }

        let product_id = product_cfg.product.id.clone();
        let collect_data = if let Some(specific_id) = &cfg.args.product_id {
            if specific_id == &product_id {
                if let Some(base) = &cfg.args.product_base {
                    product_cfg.product.base = Some(base.clone());
                }
                if let Some(version) = &cfg.args.product_version {
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
            collection
                .collect_product(product_cfg)
                .await
                .with_context(|| format!("Failed to collect product '{}'", product_id))?
        }
    }

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
