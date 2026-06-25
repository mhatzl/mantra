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
    let db = db::MantraDb::new(cfg.db.url.as_deref())
        .await
        .map_err(MantraError::db_setup_error)?;
    let cfg_file: MantraConfigFile = async_deserialize_from_path(&cfg.config_filepath)
        .await
        .map_err(MantraError::cfg_error)?;
    cfg_file
        .inheritable_product_cfg
        .check_validity()
        .map_err(MantraError::cfg_error)?;

    match cfg.cmd {
        cmd::Cmd::Report(args) => cmd::report::report(
            &db,
            ReportConfig::new(
                cfg.config_filepath,
                cfg_file,
                args,
                ReportEnvironmentVariables {},
            )
            .map_err(MantraError::cfg_error)?,
        )
        .await
        .map_err(MantraError::report_error)?,
        cmd::Cmd::Collect(args) => {
            let mut product_map = HashSet::new();

            for product_cfg in cfg_file.products {
                let mut product = product_cfg
                    .product
                    .to_product(&cfg_file.inheritable_product_cfg)
                    .map_err(MantraError::cfg_error)?;

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
                    .map_err(MantraError::collect_error)?
                }
            }
        }
        cmd::Cmd::Prune => todo!(),
        cmd::Cmd::Clear => todo!(),
    }

    db.close().await;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("Error: {}{}", .kind, if let Some(source) = .source {
    format!("\n\nCaused by:\n{:?}", source)
} else { String::new() })]
pub struct MantraError {
    kind: MantraErrorKind,
    source: Option<anyhow::Error>,
}

impl MantraError {
    pub fn without_source(kind: MantraErrorKind) -> Self {
        Self { kind, source: None }
    }

    pub fn with_source(kind: MantraErrorKind, source: impl Into<anyhow::Error>) -> Self {
        Self {
            kind,
            source: Some(source.into()),
        }
    }

    pub fn db_setup_error(source: impl Into<anyhow::Error>) -> Self {
        Self::with_source(MantraErrorKind::DbSetup, source)
    }

    pub fn collect_error(source: impl Into<anyhow::Error>) -> Self {
        Self::with_source(MantraErrorKind::Collect, source)
    }

    pub fn report_error(source: impl Into<anyhow::Error>) -> Self {
        Self::with_source(MantraErrorKind::Report, source)
    }

    pub fn cfg_error(source: impl Into<anyhow::Error>) -> Self {
        Self::with_source(MantraErrorKind::Cfg, source)
    }

    pub fn kind(&self) -> MantraErrorKind {
        self.kind
    }

    pub fn source(&self) -> Option<&anyhow::Error> {
        self.source.as_ref()
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum MantraErrorKind {
    #[error("Failed to setup the database for mantra.")]
    DbSetup,
    #[error("Failed to collect mantra data.")]
    Collect,
    #[error("Failed to create the report.")]
    Report,
    #[error("Failed to read the mantra config file.")]
    Cfg,
}
