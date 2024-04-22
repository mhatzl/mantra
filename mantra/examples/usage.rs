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
        db,
        cmd: mantra::cmd::Cmd::Trace(mantra::cmd::trace::Config {
            root: root.clone(),
            project_name,
        }),
    };

    mantra::run(extract_cfg).await;

    mantra::run(project_cfg).await;

    mantra::run(trace_cfg).await;
}
