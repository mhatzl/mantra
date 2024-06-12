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

    let wiki_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Requirements(mantra::cmd::requirements::Format::FromWiki(
            mantra::cmd::requirements::WikiConfig {
                root: root.clone(),
                link: "https://github.com/mhatzl/mantra/tree/main".to_string(),
                major_version: Some(0),
            },
        )),
    };
    let req_schema_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Requirements(mantra::cmd::requirements::Format::FromSchema {
            filepath: PathBuf::from("mantra/examples/reqs.json"),
        }),
    };
    let trace_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Trace(mantra::cmd::trace::TraceKind::FromSource(
            mantra::cmd::trace::SourceConfig {
                root,
                keep_path_absolute: false,
            },
        )),
    };
    let coverage_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Coverage(mantra::cmd::coverage::Config {
            data_file: PathBuf::from("mantra/examples/coverage.json"),
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
                name: Some("mantra".to_string()),
                version: Some("1.0.1".to_string()),
                link: Some("https://github.com/mhatzl/mantra".to_string()),
            },
            tag: mantra::cmd::report::Tag {
                name: Some("0.1.0".to_string()),
                link: Some("https://github.com/mhatzl/mantra-wiki".to_string()),
            },
            info_template: Some(PathBuf::from("mantra/examples/custom_info.html")),
            test_run_template: Some(PathBuf::from("mantra/examples/test_run_meta.html")),
        }),
    };

    mantra::run(wiki_cfg).await.unwrap();

    mantra::run(req_schema_cfg).await.unwrap();

    mantra::run(trace_cfg).await.unwrap();

    mantra::run(coverage_cfg).await.unwrap();

    mantra::run(review_cfg).await.unwrap();

    mantra::run(report_cfg).await.unwrap();
}
