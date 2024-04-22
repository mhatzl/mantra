use cmd::{coverage::CoverageError, extract::ExtractError, trace::TraceError};
use db::DbError;

pub mod cfg;
pub mod cmd;
pub mod db;

#[derive(Debug, thiserror::Error)]
pub enum MantraError {
    #[error("Failed to setup the database for mantra. Cause: {}", .0)]
    DbSetup(DbError),
    #[error("Failed to update trace data. Cause: {}", .0)]
    Trace(TraceError),
    #[error("Failed to extract requirements. Cause: {}", .0)]
    Extract(ExtractError),
    #[error("Failed to add a new project. Cause: {}", .0)]
    AddProject(DbError),
    #[error("Failed to update coverage data. Cause: {}", .0)]
    Coverage(CoverageError),
}

pub async fn run(cfg: cfg::Config) -> Result<(), MantraError> {
    let db = db::MantraDb::new(&cfg.db)
        .await
        .map_err(MantraError::DbSetup)?;

    match cfg.cmd {
        cmd::Cmd::Trace(trace_cfg) => cmd::trace::trace(&db, &trace_cfg)
            .await
            .map_err(MantraError::Trace),
        cmd::Cmd::Extract(extract_cfg) => cmd::extract::extract(&db, &extract_cfg)
            .await
            .map_err(MantraError::Extract),
        cmd::Cmd::AddProject(project_cfg) => db
            .add_project(&project_cfg.name, project_cfg.origin.clone())
            .await
            .map_err(MantraError::AddProject),
        cmd::Cmd::Coverage(coverage_cfg) => {
            cmd::coverage::coverage_from_path(&coverage_cfg.data_file, &db, &coverage_cfg.cfg)
                .await
                .map_err(MantraError::Coverage)
        }
    }
}
