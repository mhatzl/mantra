// use cfg::MantraConfigPath;
// use cmd::{
//     coverage_old::CoverageError, report_old::ReportError, requirements_old::RequirementsError,
//     review_old::ReviewError, trace_old::TraceError,
// };
// use db::DbError;

use std::collections::HashSet;

use crate::{
    cfg::MantraConfigFile,
    cmd::{
        collect::cfg::{CollectConfig, CollectEnvironmentVariables},
        report::cfg::{ReportConfig, ReportEnvironmentVariables},
    },
    io::async_deserialize_from_path,
};

pub mod cfg;
pub mod cmd;
pub mod db;
mod io;

pub async fn run(cfg: cfg::CliConfig) -> Result<(), MantraError> {
    let db = db::MantraDb::new(cfg.db.url.as_deref()).await?;
    let cfg_file: MantraConfigFile = async_deserialize_from_path(&cfg.config_filepath)
        .await
        .map_err(MantraError::Cfg)?;
    cfg_file
        .inheritable_product_cfg
        .check_validity()
        .map_err(MantraError::Cfg)?;

    match cfg.cmd {
        cmd::Cmd::Report(args) => cmd::report::report(
            &db,
            ReportConfig {
                cfg_filepath: cfg.config_filepath,
                args,
                envs: ReportEnvironmentVariables {},
            },
        )
        .await
        .map_err(MantraError::Report)?,
        cmd::Cmd::Collect(args) => {
            let mut product_map = HashSet::new();

            for product_cfg in cfg_file.products {
                let mut product = product_cfg
                    .product
                    .to_product(&cfg_file.inheritable_product_cfg)
                    .map_err(MantraError::Cfg)?;

                if !product_map.insert(product.id.clone()) {
                    log::warn!(
                        "Product '{}' has more than one product entry that maps to it!",
                        &product.id
                    );
                }

                let collect_data = if let Some(specific_id) = &args.product_id {
                    if specific_id == &product.id {
                        if let Some(base) = &args.product_base {
                            product.base = Some(base.clone());
                        }
                        if let Some(version) = &args.product_version {
                            product.version = Some(version.clone());
                        }

                        true
                    } else {
                        false
                    }
                } else {
                    true
                };

                if collect_data {
                    cmd::collect::collect(
                        &db,
                        CollectConfig {
                            cfg_filepath: cfg.config_filepath.clone(),
                            args: args.clone(),
                            envs: CollectEnvironmentVariables {},
                            product,
                            requirements: product_cfg.requirements,
                            annotations: product_cfg.annotations,
                            test_runs: product_cfg.test_runs,
                            reviews: product_cfg.reviews,
                            lsif: product_cfg.lsif,
                        },
                    )
                    .await
                    .map_err(MantraError::Collect)?
                }
            }
        }
        cmd::Cmd::Prune => todo!(),
        cmd::Cmd::Clear => todo!(),
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum MantraError {
    #[error("Failed to setup the database for mantra. Cause: {}", .0)]
    DbSetup(#[from] db::DbError),
    #[error("Failed to collect mantra data. Cause: {}", .0)]
    Collect(anyhow::Error),
    #[error("Failed to create the report. Cause: {}", .0)]
    Report(anyhow::Error),
    #[error("Failed to read the mantra config file. Cause: {}", .0)]
    Cfg(anyhow::Error),
}
