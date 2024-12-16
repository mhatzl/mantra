use cfg::MantraConfigPath;
use cmd::{
    coverage::CoverageError, report::ReportError, requirements::RequirementsError,
    review::ReviewError, trace::TraceError,
};
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
    Extract(RequirementsError),
    #[error("Failed to add a new project. Cause: {}", .0)]
    AddProject(DbError),
    #[error("Failed to update coverage data. Cause: {}", .0)]
    Coverage(CoverageError),
    #[error("Failed to deprecate requirements. Cause: {}", .0)]
    DeprecateReq(DbError),
    #[error("Failed to add manual requirements. Cause: {}", .0)]
    AddManualReq(DbError),
    #[error("Failed to delete database entries. Cause: {}", .0)]
    Delete(DbError),
    #[error("Failed to add reviews. Cause: {}", .0)]
    Review(ReviewError),
    #[error("Failed to create the report. Cause: {}", .0)]
    Report(ReportError),
    #[error("Failed to collect mantra data. Cause: {}", .0)]
    Collect(String),
    #[error("Failed to prune the database. Cause: {}", .0)]
    Prune(DbError),
    #[error("Failed to clear the database. Cause: {}", .0)]
    Clear(DbError),
}

pub async fn run(cfg: cfg::Config) -> Result<(), MantraError> {
    let db = db::MantraDb::new(&cfg.db)
        .await
        .map_err(MantraError::DbSetup)?;

    match cfg.cmd {
        cmd::Cmd::Report(report_cfg) => cmd::report::report(&db, report_cfg.to_cfg().await)
            .await
            .map_err(MantraError::Report),
        cmd::Cmd::Collect(collect_cfg) => collect(&db, collect_cfg).await,
        cmd::Cmd::Prune => db.prune().await.map_err(MantraError::Prune),
        cmd::Cmd::Clear => db.clear().await.map_err(MantraError::Clear),
    }
}

async fn collect(db: &db::MantraDb, cfg: MantraConfigPath) -> Result<(), MantraError> {
    let collect_cfg = tokio::fs::read_to_string(&cfg.filepath)
        .await
        .map_err(|_| {
            MantraError::Collect(format!("Could not read file '{}'.", cfg.filepath.display()))
        })?;
    let collect_file: cfg::MantraConfigFile = toml::from_str(&collect_cfg).map_err(|err| {
        MantraError::Collect(format!(
            "Could not read the TOML configuration. Cause: {}",
            err
        ))
    })?;

    cmd::requirements::collect(db, &collect_file.requirements)
        .await
        .map_err(MantraError::Extract)?;

    cmd::trace::collect(db, &collect_file.traces)
        .await
        .map_err(MantraError::Trace)?;

    if let Some(coverage) = collect_file.coverage {
        for file in coverage.files {
            let coverage_changes = cmd::coverage::collect_from_path(db, &file)
                .await
                .map_err(MantraError::Coverage)?;

            println!("{coverage_changes}");
        }
    }

    if let Some(review) = collect_file.review {
        let added_review_cnt = cmd::review::collect(db, review)
            .await
            .map_err(MantraError::Review)?;

        if added_review_cnt == 0 {
            println!("No review was added.");
        } else {
            println!("Added '{}' reviews.", added_review_cnt);
        }
    }

    Ok(())
}
