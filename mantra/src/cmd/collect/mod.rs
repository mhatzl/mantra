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
// pub mod lsif;
pub mod markup;
pub mod mixed_aggregation;
pub mod product_collection;
pub mod products;
pub mod requirements;
pub mod reviews;
pub mod test_runs;
pub mod walker;

#[cfg(test)]
mod test_setup;

pub async fn collect(db: &MantraDb, cfg: CollectConfig) -> Result<(), anyhow::Error> {
    let mut collection = Collection::new(db, &cfg)
        .await
        .context("Failed to start a new collection")?;

    collection.collect(cfg).await?;

    collection
        .commit()
        .await
        .context("Failed to commit the collected data")?;

    Ok(())
}
