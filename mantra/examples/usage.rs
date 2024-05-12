use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let _ = std::fs::remove_file("usage.db");
    let db = mantra::db::Config {
        url: Some("sqlite://mantra/examples/usage.db?mode=rwc".to_string()),
    };
    let root = PathBuf::from("mantra/examples/usage/");

    let extract_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Extract(mantra::cmd::extract::Config {
            root: root.clone(),
            link: "local".to_string(),
            origin: mantra::cmd::extract::ExtractOrigin::GitHub,
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
    let report_cfg = mantra::cfg::Config {
        db,
        cmd: mantra::cmd::Cmd::Report(mantra::cmd::report::ReportConfig {
            path: PathBuf::from("./"),
            template: None,
            json: true,
        }),
    };

    mantra::run(extract_cfg).await.unwrap();

    mantra::run(trace_cfg).await.unwrap();

    mantra::run(coverage_cfg).await.unwrap();

    // mantra::run(report_cfg).await.unwrap();
}