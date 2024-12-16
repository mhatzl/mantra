use std::path::PathBuf;

use mantra::{
    cfg::{MantraConfigPath, Project},
    cmd::report::{ReportFormat, ReportTemplate},
};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let _ = std::fs::remove_file("mantra/examples/usage.db");
    let db = mantra::db::Config {
        url: Some("sqlite://mantra/examples/usage.db?mode=rwc".to_string()),
    };
    let mantra_file: PathBuf = "mantra/examples/mantra.toml".into();

    let report_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Report(Box::new(mantra::cmd::report::ReportCliConfig {
            path: PathBuf::from("mantra/examples/mantra_report.html"),
            mantra_config: Some(mantra_file.clone()),
            template: ReportTemplate::default(),
            formats: vec![ReportFormat::Json, ReportFormat::Html],
            project: Project::default(),
            tag: mantra::cmd::report::Tag {
                name: Some("0.1.0".to_string()),
                link: Some("https://github.com/mhatzl/mantra-wiki".to_string()),
            },
        })),
    };

    let collect_cfg = mantra::cfg::Config {
        db,
        cmd: mantra::cmd::Cmd::Collect(MantraConfigPath {
            filepath: mantra_file,
        }),
    };

    mantra::run(collect_cfg).await.unwrap();

    mantra::run(report_cfg).await.unwrap();
}
