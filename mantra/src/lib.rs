pub mod cfg;
pub mod cmd;
pub mod db;

pub async fn run(cfg: cfg::Config) {
    let db = db::MantraDb::new(&cfg.db).await.unwrap();

    match cfg.cmd {
        cmd::Cmd::Trace(trace_cfg) => {
            cmd::trace::trace(&db, &trace_cfg).await.unwrap();
        }
        cmd::Cmd::Extract(extract_cfg) => {
            cmd::extract::extract(&db, &extract_cfg).await.unwrap();
        }
        cmd::Cmd::AddProject(project_cfg) => {
            db.add_project(&project_cfg.name, project_cfg.origin.clone())
                .await
                .unwrap();
        }
    }
}
