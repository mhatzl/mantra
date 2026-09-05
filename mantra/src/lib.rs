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

#[cfg(feature = "macros")]
pub use mantra_macros as macros;

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
        cmd::Cmd::Collect(args) => cmd::collect::collect(
            &db,
            CollectConfig::new(
                cfg.config_filepath,
                cfg_file,
                args,
                CollectEnvironmentVariables {},
            )
            .map_err(MantraError::cfg_error)?,
        )
        .await
        .map_err(MantraError::collect_error)?,
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
