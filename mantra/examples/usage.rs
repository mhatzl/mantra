use std::path::PathBuf;

use mantra::cfg::ProjectConfig;
use mantra::db::GitRepoOrigin;

#[tokio::main]
async fn main() {
    let db = mantra::db::Config {
        url: Some("sqlite://mantra/examples/usage.db?mode=rwc".to_string()),
    };
    let root = PathBuf::from("mantra/examples/usage/");
    let project_name = "usage".to_string();

    let extract_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Extract(mantra::cmd::extract::Config {
            root: root.clone(),
            link: "local".to_string(),
            origin: mantra::cmd::extract::ExtractOrigin::GitHub,
        }),
    };
    let project_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::AddProject(ProjectConfig {
            name: project_name.clone(),
            origin: mantra::db::ProjectOrigin::GitRepo(GitRepoOrigin {
                link: "local".to_string(),
                branch: None,
            }),
        }),
    };
    let trace_cfg = mantra::cfg::Config {
        db: db.clone(),
        cmd: mantra::cmd::Cmd::Trace(mantra::cmd::trace::Config {
            root: root.clone(),
            project_name: project_name.clone(),
        }),
    };
    let coverage_cfg = mantra::cfg::Config {
        db,
        cmd: mantra::cmd::Cmd::Coverage(mantra::cmd::coverage::CliConfig {
            data_file: PathBuf::from("mantra/examples/usage/defmt_test.log"),
            cfg: mantra::cmd::coverage::Config {
                project_name,
                root,
                test_prefix: None,
                fmt: mantra::cmd::coverage::LogFormat::DefmtJson,
            },
        }),
    };

    mantra::run(extract_cfg).await.unwrap();

    mantra::run(project_cfg).await.unwrap();

    mantra::run(trace_cfg).await.unwrap();

    mantra::run(coverage_cfg).await.unwrap();
}
