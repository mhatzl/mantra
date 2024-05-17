use cmd::{coverage::CoverageError, extract::ExtractError, report::ReportError, trace::TraceError};
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
    #[error("Failed to create the report.")]
    Report(ReportError),
    #[error("Failed to clean the database. Cause: {}", .0)]
    Clean(DbError),
}

pub async fn run(cfg: cfg::Config) -> Result<(), MantraError> {
    let db = db::MantraDb::new(&cfg.db)
        .await
        .map_err(MantraError::DbSetup)?;

    let project = get_project(&cfg);

    match cfg.cmd {
        cmd::Cmd::Trace(trace_cfg) => {
            let changes = cmd::trace::trace(&db, &trace_cfg)
                .await
                .map_err(MantraError::Trace)?;

            println!("{changes}");

            Ok(())
        }
        cmd::Cmd::Extract(extract_cfg) => {
            let changes = cmd::extract::extract(&db, &project, &extract_cfg)
                .await
                .map_err(MantraError::Extract)?;

            println!("{changes}");

            Ok(())
        }
        cmd::Cmd::Coverage(coverage_cfg) => {
            cmd::coverage::coverage_from_path(&coverage_cfg.data_file, &db, &coverage_cfg.cfg)
                .await
                .map_err(MantraError::Coverage)
        }
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
        cmd::Cmd::Report(report_cfg) => cmd::report::report(&db, &project, report_cfg)
            .await
            .map_err(MantraError::Report),
        cmd::Cmd::Clean => db.clean().await.map_err(MantraError::Clean),
    }
}

fn get_project(cfg: &cfg::Config) -> cfg::Project {
    if cfg.cargo {
        cfg::Project {
            name: std::env::var("CARGO_PKG_NAME").unwrap_or("unknown".to_string()),
            full_version: std::env::var("CARGO_PKG_VERSION").unwrap_or("unknown".to_string()),
            major_version: std::env::var("CARGO_PKG_VERSION_MAJOR")
                .ok()
                .map(|s| s.parse().expect("Major version must be integer.")),
            link: std::env::var("CARGO_PKG_REPOSITORY").ok(),
        }
    } else {
        let major_version = match &cfg.project_version {
            Some(version) => match version.split_once('.') {
                Some((mv, _)) => mv.parse().ok(),
                None => None,
            },
            None => None,
        };

        cfg::Project {
            name: cfg.project_name.clone().unwrap_or("unknown".to_string()),
            full_version: cfg.project_version.clone().unwrap_or("unknown".to_string()),
            major_version,
            link: cfg.project_link.clone(),
        }
    }
}
