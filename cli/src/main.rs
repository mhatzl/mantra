use std::path::PathBuf;

use mantra_db::MantraDb;

#[tokio::main]
async fn main() {
    let db_cfg = mantra_db::Config {
        url: Some("sqlite:mantra.db".to_string()),
    };
    let db = MantraDb::new(&db_cfg).await.unwrap();

    let extract_cfg = mantra_extract::Config {
        root: PathBuf::from(r"../mantra-wiki/5-Requirements"),
        link: "test".to_string(),
        origin: mantra_extract::ExtractOrigin::GitHub,
    };
    mantra_extract::extract(&db, &extract_cfg).await.unwrap();
}

pub struct Config {
    pub db: ConfigDb,
    pub trace: ConfigTrace,
    pub extract: ConfigExtract,
}

pub struct ConfigDb {
    /// URL to connect to a SQL database.
    /// Default is a SQLite file named `mantra.db` that is located under `.mantra/` at the workspace root.
    pub url: Option<String>,
}

pub struct ConfigTrace {
    pub project_name: String,
    pub root_dir: PathBuf,
}

pub struct ConfigExtract {
    pub root_dir: PathBuf,
    pub origin_kind: ExtractOriginKind,
}

pub enum ExtractOriginKind {
    GitHub,
    Jira,
}
