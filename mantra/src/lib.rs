use cmd::{coverage::CoverageError, extract::ExtractError, trace::TraceError};
use db::DbError;

pub mod cfg;
pub mod cmd;
pub mod db;
pub mod path;

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
    #[error("Failed to deprecate requirements. Cause: {}", .0)]
    DeprecateReq(DbError),
    #[error("Failed to add untraceable requirements. Cause: {}", .0)]
    AddManualReq(DbError),
    #[error("Failed to delete database entries. Cause: {}", .0)]
    Delete(DbError),
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
        cmd::Cmd::Coverage(coverage_cfg) => {
            cmd::coverage::coverage_from_path(&coverage_cfg.data_file, &db, &coverage_cfg.cfg)
                .await
                .map_err(MantraError::Coverage)
        }
        cmd::Cmd::DeprecateReq(deprecate_cfg) => {
            for req_id in deprecate_cfg.req_ids {
                db.add_deprecated(&req_id)
                    .await
                    .map_err(MantraError::DeprecateReq)?;
            }

            Ok(())
        }
        cmd::Cmd::AddManuelReq(manual_req_cfg) => {
            for req_id in manual_req_cfg.req_ids {
                db.add_manual_req(&req_id)
                    .await
                    .map_err(MantraError::AddManualReq)?;
            }

            Ok(())
        }
        cmd::Cmd::DeleteReqs(delete_req_cfg) => db
            .delete_reqs(&delete_req_cfg)
            .await
            .map_err(MantraError::Delete),
        cmd::Cmd::DeleteTraces(delete_traces_cfg) => db
            .delete_traces(&delete_traces_cfg)
            .await
            .map_err(MantraError::Delete),
        cmd::Cmd::DeleteCoverage(delete_coverage_cfg) => db
            .delete_coverage(&delete_coverage_cfg)
            .await
            .map_err(MantraError::Delete),
        cmd::Cmd::DeleteDeprecated(delete_deprecated_cfg) => db
            .delete_deprecated(&delete_deprecated_cfg)
            .await
            .map_err(MantraError::Delete),
        cmd::Cmd::DeleteManualReq(delete_manual_req_cfg) => db
            .delete_manual_reqs(&delete_manual_req_cfg)
            .await
            .map_err(MantraError::Delete),
        cmd::Cmd::DeleteReview(_delete_review_cfg) => todo!(),
    }
}
