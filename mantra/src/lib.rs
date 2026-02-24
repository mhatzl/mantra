// use cfg::MantraConfigPath;
// use cmd::{
//     coverage_old::CoverageError, report_old::ReportError, requirements_old::RequirementsError,
//     review_old::ReviewError, trace_old::TraceError,
// };
// use db::DbError;

use crate::{
    cfg::MantraConfigFile,
    cmd::collect::cfg::{CollectArguments, CollectConfig, CollectEnvironmentVariables},
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

    for product_cfg in cfg_file.products {
        match cfg.cmd {
            cmd::Cmd::Report => todo!(),
            cmd::Cmd::Collect => cmd::collect::collect(
                &db,
                CollectConfig {
                    cfg_filepath: cfg.config_filepath.clone(),
                    args: CollectArguments {
                        replace_hashed: false,
                    },
                    envs: CollectEnvironmentVariables {},
                    product: product_cfg.product,
                    requirements: product_cfg.requirements,
                    annotations: product_cfg.annotations,
                    test_runs: product_cfg.test_runs,
                    reviews: product_cfg.reviews,
                    lsif: product_cfg.lsif,
                },
            )
            .await
            .map_err(MantraError::Collect)?,
            cmd::Cmd::Prune => todo!(),
            cmd::Cmd::Clear => todo!(),
        }
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
