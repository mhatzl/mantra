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
    #[error("Failed to clean the database. Cause: {}", .0)]
    Clean(DbError),
}

pub async fn run(cfg: cfg::Config) -> Result<(), MantraError> {
    let db = db::MantraDb::new(&cfg.db)
        .await
        .map_err(MantraError::DbSetup)?;

    match cfg.cmd {
        cmd::Cmd::Trace(trace_kind) => {
            let changes = cmd::trace::collect(&db, trace_kind)
                .await
                .map_err(MantraError::Trace)?;

            println!("{changes}");

            Ok(())
        }
        cmd::Cmd::Requirements(extract_cfg) => {
            let changes = cmd::requirements::collect(&db, &extract_cfg)
                .await
                .map_err(MantraError::Extract)?;

            println!("{changes}");

            Ok(())
        }
        cmd::Cmd::Coverage(coverage_cfg) => {
            cmd::coverage::collect_from_path(&coverage_cfg.data_file, &db)
                .await
                .map_err(MantraError::Coverage)
        }
        cmd::Cmd::DeleteOld(delete_old_cfg) => db
            .delete_old_generations(delete_old_cfg.clean)
            .await
            .map_err(MantraError::Delete),
        cmd::Cmd::DeleteReqs(delete_req_cfg) => db
            .delete_reqs(&delete_req_cfg)
            .await
            .map_err(MantraError::Delete),
        cmd::Cmd::DeleteTraces(delete_traces_cfg) => db
            .delete_traces(&delete_traces_cfg)
            .await
            .map_err(MantraError::Delete),
        cmd::Cmd::DeleteTestRuns(delete_test_runs_cfg) => db
            .delete_test_runs(delete_test_runs_cfg)
            .await
            .map_err(MantraError::Delete),
        cmd::Cmd::DeleteReviews(delete_reviews_cfg) => db
            .delete_reviews(delete_reviews_cfg)
            .await
            .map_err(MantraError::Delete),
        cmd::Cmd::Report(report_cfg) => cmd::report::report(&db, report_cfg)
            .await
            .map_err(MantraError::Report),
        cmd::Cmd::Review(review_cfg) => {
            let added_review_cnt = cmd::review::collect(&db, review_cfg)
                .await
                .map_err(MantraError::Review)?;

            println!("Added '{}' reviews.", added_review_cnt);

            Ok(())
        }
        cmd::Cmd::Clean => db.clean().await.map_err(MantraError::Clean),
    }
}
