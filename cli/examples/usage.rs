use std::path::PathBuf;

use mantra::ProjectConfig;
use mantra_db::GitRepoOrigin;

#[tokio::main]
async fn main() {
    let db = mantra_db::Config {
        url: Some("sqlite://cli/examples/usage.db?mode=rwc".to_string()),
    };
    let root = PathBuf::from("cli/examples/usage/");

    let extract_cfg = mantra::Config {
        db: db.clone(),
        cmd: mantra::Cmd::Extract(mantra_extract::Config {
            root: root.clone(),
            link: "local".to_string(),
            origin: mantra_extract::ExtractOrigin::GitHub,
        }),
    };
    let project_cfg = mantra::Config {
        db: db.clone(),
        cmd: mantra::Cmd::AddProject(ProjectConfig {
            name: "usage".to_string(),
            origin: mantra_db::ProjectOrigin::GitRepo(GitRepoOrigin {
                link: "local".to_string(),
                branch: None,
            }),
        }),
    };
    let trace_cfg = mantra::Config {
        db,
        cmd: mantra::Cmd::Trace(mantra_trace::Config {
            root: root.clone(),
            project_name: "usage".to_string(),
        }),
    };

    mantra::run(extract_cfg).await;

    mantra::run(project_cfg).await;

    mantra::run(trace_cfg).await;
}
