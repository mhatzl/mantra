use mantra_db::{MantraDb, ProjectOrigin};

#[derive(clap::Parser)]
pub struct Config {
    #[command(flatten)]
    pub db: mantra_db::Config,
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(clap::Subcommand)]
pub enum Cmd {
    Trace(mantra_trace::Config),
    Extract(mantra_extract::Config),
    AddProject(ProjectConfig),
}

#[derive(clap::Args)]
pub struct ProjectConfig {
    pub name: String,
    #[command(subcommand)]
    pub origin: ProjectOrigin,
}

pub async fn run(cfg: Config) {
    let db = MantraDb::new(&cfg.db).await.unwrap();

    match cfg.cmd {
        Cmd::Trace(trace_cfg) => {
            mantra_trace::trace(&db, &trace_cfg).await.unwrap();
        }
        Cmd::Extract(extract_cfg) => {
            mantra_extract::extract(&db, &extract_cfg).await.unwrap();
        }
        Cmd::AddProject(project_cfg) => {
            db.add_project(&project_cfg.name, project_cfg.origin.clone())
                .await
                .unwrap();
        }
    }
}
