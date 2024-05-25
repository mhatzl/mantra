use std::path::PathBuf;

use mantra::cmd::report::{Project, ReportFormat};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .init();

    let _ = std::fs::remove_file("mantra/examples/usage.db");
    let db = mantra::db::Config {
        url: Some("sqlite://mantra/examples/usage.db?mode=rwc".to_string()),
    };
    let root = PathBuf::from("mantra/examples/usage/");

    let extract_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Extract(mantra::cmd::extract::Config {
            root: root.clone(),
            link: "https://github.com/mhatzl/mantra/tree/macros".to_string(),
            origin: mantra::cmd::extract::ExtractOrigin::GitHub,
            major_version: Some(0),
        }),
    };
    let trace_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Trace(mantra::cmd::trace::Config {
            root,
            keep_root_absolute: false,
        }),
    };
    let coverage_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Coverage(mantra::cmd::coverage::CliConfig {
            data_file: PathBuf::from("mantra/examples/usage/defmt_test.log"),
            cfg: mantra::cmd::coverage::Config {
                test_run: "test-run".to_string(),
                fmt: mantra::cmd::coverage::LogFormat::DefmtJson,
            },
        }),
    };
    let review_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Review(mantra::cmd::review::ReviewConfig {
            reviews: vec![PathBuf::from("mantra/examples/my_review.toml")],
        }),
    };
    let report_cfg = mantra::cfg::Config {
        db,
        cmd: mantra::cmd::Cmd::Report(mantra::cmd::report::ReportConfig {
            path: PathBuf::from("mantra/examples/mantra_report.html"),
            template: None,
            formats: vec![ReportFormat::Json, ReportFormat::Html],
            project: Project {
                project_name: Some("mantra".to_string()),
                project_version: Some("1.0.1".to_string()),
                project_link: Some("https://github.com/mhatzl/mantra".to_string()),
            },
        }),
    };

    mantra::run(extract_cfg).await.unwrap();

    mantra::run(trace_cfg).await.unwrap();

    mantra::run(coverage_cfg).await.unwrap();

    mantra::run(review_cfg).await.unwrap();

    mantra::run(report_cfg).await.unwrap();
}
